# 開発・ビルド・リリース

## 必要環境

- Node.js 20 以上 / npm
- Rust（stable）
- Tauri の[各 OS 向け前提パッケージ](https://tauri.app/start/prerequisites/)

現時点で環境変数や外部 API キーは必要ありません。

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
npm run build
```

Rust のテスト:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

現時点では E2E テストはありません。変更内容に応じて、`npm run tauri dev` でタブ操作、戻る・進む・再読み込み、外部リンクの確認ダイアログも手動確認してください。

## ローカルビルド

```bash
npm run tauri build
```

macOS で QA 用に最新の `.app` を `dist-local/` へ同期する場合:

```bash
npm run dist:local
```

（`dist-local/` は `.gitignore` 対象です）

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

コード署名は行いません。利用者向けの未署名アプリ起動手順はルートの [README.md](../README.md) に記載しています。
