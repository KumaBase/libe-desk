# 開発・ビルド・リリース

## 必要環境

- Node.js 20 以上 / npm
- Rust（stable）
- Tauri の[各 OS 向け前提パッケージ](https://tauri.app/start/prerequisites/)

通常の開発では環境変数や外部 API キーは必要ありません。macOS の署名・公証には以下の設定が必要です。

## セットアップ

```bash
git clone https://github.com/KumaBase/libe-desk.git
cd libe-desk
npm ci
```

`package-lock.json` に記録された依存関係を再現するため、通常は `npm install` ではなく `npm ci` を使用します。

## 開発

```bash
npm run tauri dev
```

起動後、リベシティ本体が表示され、サイドバーからサービスを開けることを確認してください。

## 検証

型検査とフロントエンドのビルド:

```bash
npm run typecheck
npm test
npm run build
```

Rust のテスト:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

`npm test` は jsdom 上でお気に入りの注入スクリプトを検証します。500件のリンクがある一覧で無関係なDOM更新が全体検索を起こさないこと、追加・再利用されたリンク、SPA遷移、非表示からの復帰、お気に入りの反映を確認します。実ページの読み込み時間を測るテストではありません。

性能改修後の手動確認項目:

- サイドバーで同じサービス・お気に入りを再度開くと既存タブへ切り替わり、「＋」と `⌘T` では新しいタブが増える。
- タブを切り替えるとOSメニューのチェックが追従し、タイトル変更・並べ替え・閉じる操作が反映される。
- 複数タブを開いてウィンドウをリサイズし、非表示だったタブへ切り替えても表示範囲が合う。
- チャットやつぶやきの追加読み込み、プロフィール移動で★が表示され、登録・解除できる。
- 同じページ・タブ数でCPU使用率と体感の応答を比較する。初回読み込み時間は通常ブラウザとも比較する。

2026-09-10の改修時は自動検証を実施しましたが、実行環境からmacOSの画面サービスに接続できず、上記の実機操作と実サイトの速度比較は未実施です。

現時点では E2E テストはありません。変更内容に応じて、`npm run tauri dev` でタブ操作、戻る・進む・再読み込み、外部リンクが確認なしで新しいタブに開くことも手動確認してください。

## ローカルビルド

```bash
npm run tauri build
```

macOS で QA 用に最新の `.app` を `dist-local/` へ同期する場合:

```bash
npm run dist:local
```

（`dist-local/` は `.gitignore` 対象です）

## macOS の署名・公証

Kuma Base LLC の `Developer ID Application` 証明書と対応する秘密鍵がキーチェーンにある Mac では、次のコマンドで署名付きアプリを作成できます。

```bash
npm run build:mac:signed
```

### 実行できるのは GUI セッションだけ

署名と公証は、その Mac の画面でログインしているセッション（`launchctl managername` が `Aqua`）からしか実行できません。SSH 接続やバックグラウンドのエージェントからは、キーチェーンが解錠済みでも `codesign` が `errSecInternalComponent` で失敗し、`notarytool` は公証プロファイルを `keychainLocked` として読めません。GUI ログインセッションに紐づくセキュリティセッションを持たないためで、別のターミナルで `security unlock-keychain` しても解消しません。

そのため署名リリースは Mac mini 本体のターミナルから、次のスクリプトで通して実行します。署名ビルドから公証、チケット添付、DMG 作成、チェックサム生成までを行い、`dist-local/` へ配布物を出します。GitHub Releases への添付は行いません。

```bash
bash scripts/release-macos-signed.sh
```

スクリプトは冒頭でセッション種別、アーキテクチャ、証明書、公証プロファイルを検査して、条件を満たさなければ何もせず止まります。`--skip-checks` でリリース前チェック（`version:check`、`npm ci`、`typecheck`、`cargo fmt --check`、`cargo test`）を省略できます。

DMG の作成以降でつまずいた場合は、`--dmg-only` で再開できます。署名・公証済みの `.app` が残っていることを検証したうえで、時間のかかるビルドと `.app` の公証をやり直さずに DMG から進めます。

```bash
bash scripts/release-macos-signed.sh --dmg-only
```

DMG の作成には `dmgbuild` が要り、これは Python 3.10 以上を要求します。`/usr/bin/python3` は 3.9 系のことがあるため、スクリプトは venv の Python 版数と `dmgbuild` / `Pillow` の import 可否まで確認し、条件を満たさなければ venv を作り直します。

なお `CARGO_TARGET_DIR` が環境変数で別の場所へ向いていることがあります（エディタのサンドボックスなど）。`build-macos-dmg.py` は `src-tauri/target/release/bundle/macos/Libe Desk.app` を決め打ちで参照するため、スクリプトは `CARGO_TARGET_DIR` をリポジトリ内へ必ず上書きします。手作業で進める場合も同じ指定が要ります。

専用設定 `src-tauri/tauri.signed.conf.json` で署名者と Hardened Runtime を指定します。通常のビルドと GitHub Actions は従来の設定を使います。秘密鍵はリポジトリに保存しません。

生成先は `src-tauri/target/release/bundle/macos/Libe Desk.app` です。署名は次で確認します。

```bash
codesign --verify --deep --strict --verbose=2 "src-tauri/target/release/bundle/macos/Libe Desk.app"
codesign -dv --verbose=4 "src-tauri/target/release/bundle/macos/Libe Desk.app"
```

**署名だけでは配布準備完了ではありません。** 公開前に Apple の公証とチケットの添付が必要です。Tauri の公証には Apple の認証設定が別途必要です。Apple Account の通常パスワードや秘密鍵をソース・ログ・チャットへ記載しないでください。

設定方法は [Tauri の署名・公証ドキュメント](https://v2.tauri.app/distribute/sign/macos/) を参照してください。公証後は `xcrun stapler validate` と `spctl --assess --type execute` でも成果物を確認します。

この Mac では公証用認証をキーチェーンプロファイル `libe-desk-notary` に保存しています。署名ビルド後、次の手順で公証できます（Apple にアプリ本体を送信します）。

```bash
ditto -c -k --keepParent "src-tauri/target/release/bundle/macos/Libe Desk.app" /private/tmp/libe-desk-notary.zip
xcrun notarytool submit /private/tmp/libe-desk-notary.zip --keychain-profile libe-desk-notary --wait
```

結果が `Accepted` になったことを確認してからチケットを添付します。

```bash
xcrun stapler staple "src-tauri/target/release/bundle/macos/Libe Desk.app"
xcrun stapler validate "src-tauri/target/release/bundle/macos/Libe Desk.app"
codesign --verify --deep --strict --verbose=2 "src-tauri/target/release/bundle/macos/Libe Desk.app"
spctl --assess --type execute --verbose=2 "src-tauri/target/release/bundle/macos/Libe Desk.app"
```

配布用 ZIP はチケット添付後の `.app` から改めて作成してください。再ビルドした場合は公証・添付・検証をやり直します。上記プロファイルはローカルキーチェーンの設定であり、他の Mac や GitHub Actions には引き継がれません。

### ドラッグしてインストールする DMG

公証チケット添付済みの Apple Silicon アプリから、案内背景と `/Applications` へのショートカットを含む DMG を作成できます。アプリを再ビルドせず、そのまま格納します。

```bash
python3 -m venv /private/tmp/libe-desk-dmg-venv
/private/tmp/libe-desk-dmg-venv/bin/pip install dmgbuild==1.6.7 Pillow==12.3.0
/private/tmp/libe-desk-dmg-venv/bin/python scripts/build-macos-dmg.py --output dist-local/Libe.Desk_0.1.1_macos_arm64.dmg
```

出力先が存在すると停止します。バージョンに合わせてファイル名を指定してください。スクリプトは DMG を署名しますが、公証は次の手順で別途行います。

```bash
xcrun notarytool submit dist-local/Libe.Desk_0.1.1_macos_arm64.dmg --keychain-profile libe-desk-notary --wait
# Accepted を確認した後に実行
xcrun stapler staple dist-local/Libe.Desk_0.1.1_macos_arm64.dmg
xcrun stapler validate dist-local/Libe.Desk_0.1.1_macos_arm64.dmg
codesign --verify --strict --verbose=2 dist-local/Libe.Desk_0.1.1_macos_arm64.dmg
```

DMG を開いて配置と Applications のリンク先を確認し、中のアプリにも署名・公証・Gatekeeper の検証を行ってください。最終 DMG のチェックサムを計算し、GitHub Releases の添付と `SHA256SUMS` を更新します。

## バージョン

初回公開は **0.1.0** です。`0.x.y` の間は [Semantic Versioning](https://semver.org/lang/ja/) に従い、次のように上げます。

| 変更の種類 | 例 |
|---|---|
| バグ修正・文言修正 | `0.1.0` → `0.1.1` |
| 後方互換のある機能追加 | `0.1.1` → `0.2.0` |
| 破壊的変更・大きな仕様変更 | `0.2.0` → `1.0.0`（将来） |

バージョン番号は次の 3 ファイルで一致させてください。

- `package.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

`package.json` を更新したあとは `npm install` を実行し、`package-lock.json` も揃えます。確認は次で行えます。

```bash
npm run version:check
```

## リリース（GitHub Actions）

`v<version>` タグの push、または `main` ブランチからの手動実行で [.github/workflows/release.yml](../.github/workflows/release.yml) が走ります。macOS / Windows / Linux 向けにビルドし、`SHA256SUMS` を含む GitHub Release の**下書き**を作成します。内容と配布物を確認してから手動で公開してください。

リリース前に次を確認してください。

1. `npm run version:check` が成功する
2. `npm ci`、`npm run build`、Rust のテストが成功する
3. macOS / Windows で主要操作を確認する（Linux はベストエフォート）
4. Release の説明に変更点、既知の問題、対応 OS を記載する
5. 配布ファイルの SHA-256 チェックサムを Release に掲載し、添付ファイルから再計算した値と一致することを確認する

タグは `v<version>` 形式です。初回公開は `v0.1.0`、次のパッチなら `v0.1.1` のように、3 ファイルのバージョンと一致させてください。ワークフローはタグとバージョンの一致を検証し、公開済みの同名 Release を置き換えません。

```bash
# 初回公開の例
git tag v0.1.0
git push origin v0.1.0

# 次のパッチリリースの例（3 ファイルを 0.1.1 に更新したあと）
git tag v0.1.1
git push origin v0.1.1
```

GitHub Actions ではまだコード署名を行いません。ローカルの署名設定だけでは CI に証明書は渡りません。0.2.2以降のWindows版（`.msi` / `.exe`）はこのワークフローの未署名ビルドで配布します。SignPath FoundationによるWindows向けコード署名への切り替えを準備中です。現在の署名・対応環境と、未署名版向けの起動手順は [README.md](../README.md) を参照してください。

Windowsのリリース前には、Windowsマシンで `npm run tauri build` を実行し、生成されたインストーラーからの起動と主要操作（タブ・サイドバー・お気に入り・ウィンドウ位置の復元）を確認してください。
