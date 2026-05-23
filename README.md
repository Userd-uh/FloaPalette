<img src="https://r2cdn.perplexity.ai/pplx-full-logo-primary-dark%402x.png" style="height:64px;margin-right:32px"/>

# 機能、インストール手順や設定例

**インストール手順・設定例・開発手順・技術構成・各言語の役割** まで含めた完全版をまとめた。Tauri の公式では SvelteKit は `@sveltejs/adapter-static` を使い、`frontendDist` は `build/` を指す構成が案内されています。[^1][^2][^3]
また、トレイ機能は Tauri v2 の system tray 機能を使い、Windows の入力注入は `windows-sys` で `SendInput` を扱うのが前提です。[^4][^5][^6]

***

# FloaPalette

FloaPalette は、GIT コマンドや Markdown ショートカットをクリックひとつで即時入力できる、Windows 向けの常駐型フローティングパレットアプリです。
システムトレイに常駐しながら常時最前面で動作し、左の一覧から項目を選ぶとアクティブなアプリへ直接文字列を送信します。

## 主な機能

- 常時最前面のフローティングウィンドー。
- システムトレイ常駐。
- 左ペインでコマンド一覧を表示。
- 右ペインで項目をその場で編集。
- 項目クリックでアクティブウィンドーへ即時入力。
- GIT / Markdown / ショートカットをカテゴリ分けして管理。
- 1ファイル JSON で設定を保存。
- Mac 風の半透明・ガラス感ある見た目。


## 想定ユースケース

- Git 作業中に `git status` や `git pull` を素早く入力したい。
- Markdown の `#`、`##`、コードブロックなどをすぐ呼び出したい。
- 使う頻度の高い短いコマンドをまとめておきたい。
- 自分用のチートシートを、ただ見るだけでなく即入力できる形で使いたい。

***

## 技術構成

### フロントエンド

- **Svelte / SvelteKit**
    - UI 構築。
    - 左右ペインの表示。
    - 入力フォーム。
    - 一覧のフィルタリング。
    - 編集状態の管理。


### バックエンド

- **Rust**
    - JSON の読み書き。
    - Tauri コマンドの実装。
    - Windows への文字送信。
    - 常駐やショートカット関連の処理。


### Windows 固有機能

- **windows-sys**
    - `SendInput` を使った文字列の自動送信。
    - Enter キー送信。


### アプリ基盤

- **Tauri v2**
    - デスクトップアプリ化。
    - 常時最前面ウィンドー。
    - システムトレイ常駐。
    - フロントと Rust の橋渡し。

***

## 各機能に使っている言語

| 機能 | 使用言語 / 技術 | 役割 |
| :-- | :-- | :-- |
| UI 表示 | Svelte / TypeScript | 左右ペイン、一覧、編集フォーム、検索 |
| 状態管理 | TypeScript | 選択中カテゴリ・項目・検索状態の管理 |
| JSON 保存 | Rust | `commands.json` の読み書き |
| 項目編集 | Svelte + TypeScript + Rust | UI で変更し、Rust 側で保存 |
| クリック即時送信 | Rust + `windows-sys` | アクティブウィンドーへ文字列入力 |
| 常時最前面 | Tauri | ウィンドーを前面維持 |
| トレイ常駐 | Tauri | アプリを閉じずに常駐 |
| グローバルショートカット | Rust + Tauri | `Ctrl+Shift+Space` などで再表示 |
| 配置・見た目 | CSS | 半透明、角丸、Mac 風 UI |
| ビルド設定 | JSON / TOML / SvelteKit 設定 | Tauri と SvelteKit の接続 |


***

## フォルダ構成

```text
FloaPalette/
├─ data/
│  └─ commands.json
├─ src/
│  ├─ lib/
│  │  └─ types.ts
│  ├─ App.svelte
│  ├─ app.css
│  └─ routes/
│     └─ +layout.ts
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  └─ src/
│     └─ main.rs
├─ package.json
├─ svelte.config.ts
├─ vite.config.ts
└─ README.md
```


***

## インストール手順

### 1. 必要なものを入れる

#### Rust

Rust は `rustup` で入れる。

```powershell
winget install Rustlang.Rustup
```

確認:

```powershell
rustc --version
cargo --version
```


#### Node.js

Svelte 側のビルドに Node.js が必要。LTS を入れる。

確認:

```powershell
node --version
npm --version
```


#### Microsoft C++ Build Tools

Tauri の Windows ビルドには Visual C++ のビルド環境が必要。
インストール時は **Desktop development with C++** を選ぶ。

#### WebView2 Runtime

Tauri の Windows 実行に必要。
Evergreen Runtime を入れる。

***

### 2. プロジェクトを作成する

対話式で作る場合:

```powershell
npm create tauri-app@latest FloaPalette
```

その後の選択は次のように進める。

- Frontend: **TypeScript / JavaScript**
- Package manager: **npm**
- Template: **Svelte**
- TypeScript: **Yes**

`Svelte` が出ない場合は、コマンドでテンプレートを直接指定して作る。

```powershell
npm create tauri-app@latest FloaPalette -- --template svelte
```


***

### 3. 依存関係を入れる

プロジェクトルートに移動して、依存関係をインストールする。

```powershell
cd D:\project\FloaPalette
npm install
```

必要に応じてフロントの API も入れる。

```powershell
npm install @tauri-apps/api
```


***

### 4. SvelteKit の静的設定を入れる

このアプリは Tauri で使うため、**SvelteKit の static adapter** を使う。

```powershell
npm install -D @sveltejs/adapter-static
```


#### `svelte.config.ts`

```ts
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter()
  }
};

export default config;
```


#### `src/routes/+layout.ts`

```ts
export const prerender = true;
export const ssr = false;
```


#### `src-tauri/tauri.conf.json`

`frontendDist` は `build/` を指すようにする。

```json
{
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../build"
  }
}
```

Tauri の公式でも SvelteKit は static adapter を使い、`frontendDist` に `build/` を指定する構成が案内されています。[^2][^3][^1]

***

### 5. Rust 側の依存を設定する

`src-tauri/Cargo.toml` に必要なクレートを入れる。

```toml
[package]
name = "floapalette"
version = "0.1.0"
description = "A Tauri App"
authors = ["you"]
edition = "2021"

[lib]
name = "floapalette_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-opener = "2"
tauri-plugin-global-shortcut = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
windows-sys = { version = "0.61", features = [
  "Win32_UI_Input_KeyboardAndMouse",
  "Win32_Foundation"
] }
```

Tauri v2 のトレイ機能は `tauri` の `tray-icon` feature を使うのが前提です。[^6][^4]
Windows の入力注入には `windows-sys` が必要です。[^5]

***

## 設定例

### `data/commands.json`

```json
{
  "appName": "CmdPal",
  "version": 1,
  "categories": [
    {
      "id": "git",
      "label": "GIT",
      "color": "#35c2a0",
      "items": [
        {
          "id": "git-status",
          "title": "status",
          "command": "git status",
          "description": "現在の状態を確認",
          "favorite": true,
          "autoEnter": false
        },
        {
          "id": "git-pull",
          "title": "pull",
          "command": "git pull",
          "description": "リモートの変更を取得",
          "favorite": false,
          "autoEnter": false
        }
      ]
    },
    {
      "id": "markdown",
      "label": "Markdown",
      "color": "#b46cff",
      "items": [
        {
          "id": "md-h1",
          "title": "H1",
          "command": "# ",
          "description": "見出し1",
          "favorite": true,
          "autoEnter": false
        },
        {
          "id": "md-code",
          "title": "code block",
          "command": "```\\n\\n```",
          "description": "コードブロック",
          "favorite": false,
          "autoEnter": false
        }
      ]
    }
  ]
}
```


***

### `src/lib/types.ts`

```ts
export type CommandItem = {
  id: string;
  title: string;
  command: string;
  description: string;
  favorite: boolean;
  autoEnter: boolean;
};

export type Category = {
  id: string;
  label: string;
  color: string;
  items: CommandItem[];
};

export type AppData = {
  appName: string;
  version: number;
  categories: Category[];
};
```


***

## 開発手順

### 1. 開発起動

まずはフロント込みで起動する。

```powershell
npm run tauri dev
```


### 2. UI調整

- `src/App.svelte`
- `src/app.css`

を編集して、左右ペインや Mac 風デザインを調整する。

### 3. Rust 側実装

- `src-tauri/src/main.rs`

で以下を実装する。

- JSON 読み書き。
- 項目送信。
- トレイ常駐。
- グローバルショートカット。
- ウィンドーの表示 / 非表示切替。


### 4. Windows 文字送信

Rust 側で `SendInput` を使い、アクティブなアプリへ文字列を送信する。

### 5. ビルド

```powershell
npm run build
cargo build
```

または Tauri 経由で本番ビルドする。

```powershell
npm run tauri build
```


***

## 動作の流れ

1. アプリを起動する。
2. トレイに常駐する。
3. 左側でカテゴリを選ぶ。
4. 項目をクリックする。
5. 文字列がアクティブなアプリへ即時送信される。
6. 必要なら右ペインで内容を編集する。
7. 編集内容は JSON に保存される。

***

## 今後の拡張候補

- 項目のドラッグ並べ替え。
- コピー専用モード。
- 貼り付け前確認モード。
- 検索の部分一致強化。
- コマンドの複数行テンプレート対応。
- テーマ切り替え。
- アイコン・ショートカットのカスタム化。

***

## ビルド前提

- Rust
- Node.js
- Microsoft C++ Build Tools
- WebView2 Runtime

***

## 補足

Tauri の system tray は v2 で公式にサポートされています。[^4][^6]
SvelteKit を使う場合は static adapter と `build/` を組み合わせる構成が公式の推奨です。[^3][^1][^2]
Windows の文字自動送信には `windows-sys` の `SendInput` を使うのが自然です。[^5]

***


<span style="display:none">[^10][^7][^8][^9]</span>

<div align="center">⁂</div>

[^1]: https://v2.tauri.app/start/frontend/sveltekit/

[^2]: https://v2.tauri.app/start/frontend-configuration/sveltekit/

[^3]: https://beta.tauri.app/start/frontend-configuration/sveltekit/

[^4]: https://v2.tauri.app/ja/learn/system-tray/

[^5]: https://docs.rs/crate/windows-sys/0.52.0

[^6]: https://v2.tauri.app/reference/javascript/api/namespacetray/

[^7]: https://github.com/tauri-apps/tauri/issues/8331

[^8]: https://docs.rs/crate/windows-sys/0.36.1/source/readme.md

[^9]: https://gist.github.com/littletsu/d1c1b512d6843071144b7b89109a8de2

[^10]: https://tauri.nodejs.cn/start/frontend/sveltekit/

