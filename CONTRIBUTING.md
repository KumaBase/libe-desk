# コントリビューションガイド

Libe Desk への提案・報告を歓迎します。このプロジェクトはリベシティ運営とは関係のない非公式プロジェクトです。

## Issue

- 不具合は、利用した OS、Libe Desk のバージョン、再現手順、期待した結果、実際の結果を記載してください。
- アカウント、契約、リベシティの掲載内容に関する問い合わせは、リベシティの公式窓口へお願いします。
- 脆弱性や機密情報を含む問題は公開 Issue に書かず、[SECURITY.md](./SECURITY.md) に従ってください。

## Pull Request

1. 変更の目的を 1 つに絞ってください。
2. [開発手順](./docs/development.md)に従ってセットアップしてください。
3. `npm run typecheck`、`npm run build`、Rustのformat・Clippy・テストを[開発手順](./docs/development.md)どおり実行してください。
4. UI や WebView の動作を変更した場合は、対象 OS と手動確認した操作を Pull Request に記載してください。
5. ユーザー向けの動作や制約が変わる場合は、README または `docs/` も更新してください。

認証情報、Cookie、トークン、個人情報を、Issue、Pull Request、ログ、スクリーンショット、テストデータへ含めないでください。

コントリビューションは、このリポジトリの [MIT License](./LICENSE) の下で提供されるものとします。
