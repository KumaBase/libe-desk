#!/usr/bin/env bash
#
# Apple Silicon 版の署名・公証済み配布物を dist-local/ に作る。
#
# キーチェーンの秘密鍵と公証プロファイルは GUI ログインセッション
# （launchctl managername が Aqua）からしか使えない。SSH や常駐エージェント
# からは codesign が errSecInternalComponent で落ちるため、Mac mini 本体の
# ターミナルから実行すること。
#
#   bash scripts/release-macos-signed.sh
#
# 生成物: dist-local/Libe.Desk_<version>_macos_arm64.{dmg,zip} と SHA256SUMS
# GitHub Releases への添付は行わない。

set -euo pipefail

NOTARY_PROFILE="libe-desk-notary"
VENV="/private/tmp/libe-desk-dmg-venv"
NOTARY_UPLOAD="/private/tmp/libe-desk-notary.zip"

skip_checks=0
allow_any_session=0
dmg_only=0
for arg in "$@"; do
  case "$arg" in
    --skip-checks) skip_checks=1 ;;
    --allow-any-session) allow_any_session=1 ;;
    # 署名・公証済みの .app が残っているとき、DMG 以降だけをやり直す。
    --dmg-only) dmg_only=1; skip_checks=1 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

step() { printf '\n=== %s\n' "$1"; }
fail() { printf 'error: %s\n' "$1" >&2; exit 1; }

step "前提の確認"

session="$(launchctl managername 2>/dev/null || echo unknown)"
echo "session: $session"
if [[ "$session" != "Aqua" && "$allow_any_session" -eq 0 ]]; then
  fail "GUI セッションではない（${session}）。Mac mini 本体のターミナルから実行すること。
どうしても続けるなら --allow-any-session を付けるが、codesign は errSecInternalComponent で失敗する見込み。"
fi

[[ "$(uname -s)" == "Darwin" ]] || fail "macOS 以外では実行できない"
[[ "$(uname -m)" == "arm64" ]] || fail "Apple Silicon 以外では配布物を作らない（uname -m: $(uname -m)）"

security find-identity -v -p codesigning | grep -q "Developer ID Application: Kuma Base LLC" \
  || fail "Developer ID Application の証明書が見つからない"

xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1 \
  || fail "公証プロファイル $NOTARY_PROFILE を読めない。キーチェーンの解錠状態を確認すること"

version="$(node -e "console.log(require('./src-tauri/tauri.conf.json').version)")"
echo "version: $version"

# build-macos-dmg.py が src-tauri/target 配下の .app を決め打ちで参照するため、
# 環境が CARGO_TARGET_DIR を別の場所へ向けていても必ず上書きする。
export CARGO_TARGET_DIR="$ROOT/src-tauri/target"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

app="$CARGO_TARGET_DIR/release/bundle/macos/Libe Desk.app"
dmg="$ROOT/dist-local/Libe.Desk_${version}_macos_arm64.dmg"
zip="$ROOT/dist-local/Libe.Desk_${version}_macos_arm64.zip"

if [[ "$skip_checks" -eq 0 ]]; then
  step "リリース前チェック"
  npm run version:check
  npm ci
  npm run typecheck
  ( cd src-tauri && cargo fmt --check && cargo test )
fi

# 公証は Apple へアプリ本体を送信する。--wait は Invalid でも 0 を返しうるので
# status を自分で見て、失敗時はログを引く。
notarize() {
  local target="$1" label="$2" payload="$3" submit status id
  step "公証: $label"
  submit="$(xcrun notarytool submit "$payload" --keychain-profile "$NOTARY_PROFILE" --wait --output-format json)"
  status="$(printf '%s' "$submit" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("status",""))')"
  id="$(printf '%s' "$submit" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("id",""))')"
  echo "status: $status (id: $id)"
  if [[ "$status" != "Accepted" ]]; then
    xcrun notarytool log "$id" --keychain-profile "$NOTARY_PROFILE" || true
    fail "$label の公証が通らなかった"
  fi
  step "チケット添付と検証: $label"
  xcrun stapler staple "$target"
  xcrun stapler validate "$target"
}

# dist-local には sync-dist-local.sh が置く検証用の .app もあるので、
# 今回作る成果物だけを消す（build-macos-dmg.py は出力先があると止まる）。
mkdir -p "$ROOT/dist-local"
rm -f "$dmg" "$ROOT/dist-local/SHA256SUMS"

if [[ "$dmg_only" -eq 1 ]]; then
  step "署名・公証済みアプリの再利用"
  [[ -d "$app" ]] || fail "アプリが見つからない: $app（--dmg-only を外して最初から実行すること）"
  codesign --verify --deep --strict --verbose=2 "$app"
  xcrun stapler validate "$app"
  spctl --assess --type execute --verbose=2 "$app"
  [[ -f "$zip" ]] || ditto -c -k --keepParent "$app" "$zip"
else
  step "署名ビルド"
  rm -f "$zip"
  npm run build:mac:signed
  [[ -d "$app" ]] || fail "アプリが見つからない: $app"

  step "署名の検証"
  codesign --verify --deep --strict --verbose=2 "$app"
  codesign -dv --verbose=4 "$app" 2>&1 | grep -E 'Authority|TeamIdentifier|Timestamp|runtime'

  # .app は直接送れないので zip に固めて送る（送信用であって配布物ではない）。
  rm -f "$NOTARY_UPLOAD"
  ditto -c -k --keepParent "$app" "$NOTARY_UPLOAD"
  notarize "$app" "Libe Desk.app" "$NOTARY_UPLOAD"
  rm -f "$NOTARY_UPLOAD"

  codesign --verify --deep --strict --verbose=2 "$app"
  spctl --assess --type execute --verbose=2 "$app"

  # 配布用 ZIP はチケットを添付した .app から作る。
  step "配布用 ZIP"
  ditto -c -k --keepParent "$app" "$zip"
fi

step "DMG の作成"
# dmgbuild は Python 3.10 以上を要求する。/usr/bin/python3 は 3.9 のことがあるので、
# venv の中身まで見て条件を満たさなければ作り直す。
if [[ ! -x "$VENV/bin/python" ]] \
  || ! "$VENV/bin/python" -c 'import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null; then
  base_python=""
  for cand in python3.13 python3.12 python3.11 python3 \
    /opt/homebrew/bin/python3.13 /opt/homebrew/bin/python3; do
    path="$(command -v "$cand" 2>/dev/null || true)"
    [[ -n "$path" ]] || continue
    if "$path" -c 'import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null; then
      base_python="$path"
      break
    fi
  done
  [[ -n "$base_python" ]] || fail "dmgbuild には Python 3.10 以上が要るが見つからなかった"
  echo "venv を作り直す: $base_python ($("$base_python" -V 2>&1))"
  rm -rf "$VENV"
  "$base_python" -m venv "$VENV"
fi
# venv があっても中身が空のことがあるので、import できるかで判定する。
if ! "$VENV/bin/python" -c 'import dmgbuild, PIL' >/dev/null 2>&1; then
  "$VENV/bin/pip" install --quiet dmgbuild==1.6.7 Pillow==12.3.0
fi
"$VENV/bin/python" scripts/build-macos-dmg.py --output "$dmg"

notarize "$dmg" "DMG" "$dmg"
codesign --verify --strict --verbose=2 "$dmg"

step "チェックサム"
( cd "$ROOT/dist-local" && shasum -a 256 "$(basename "$dmg")" "$(basename "$zip")" > SHA256SUMS && cat SHA256SUMS )

step "完了"
ls -lh "$ROOT/dist-local"
cat <<EOF

dist-local/ に署名・公証済みの配布物ができた。
DMG を開いて配置と Applications へのリンクを確認し、中のアプリも起動して主要操作を見ること。
問題なければ Cursor 側へ戻って、GitHub Releases への添付と公開を依頼する。
EOF
