# TUFF-BRG-ECO (The Unfiltered Fact Finder Bridge Ecosystem)

## 概要
- LLMのハルシネーションと現在バイアスを物理的・構造的に排除するための次世代情報検証・中継基盤。
- 計算コストの激減、事実の不動点化、およびAIの知能自壊（モデル崩壊）防止を目的とする。

## TUFF-BRG-ECO とは
ECO (Ecosystem) は、tuff-db（コア）と tuff-brg（ブリッジ）が単一のワークスペースで連携し、事実の不動点を共有する統合運用環境を指します。

## アーキテクチャ（3-Layer）
- Addon (`tuff-brg/addon/`): DOM断片を収集して Bridge に送信。セレクタ不一致時は警告バナーを表示。
- Bridge (`tuff-brg`): WebSocket受信、Hybrid判定（Fast Path -> LLM Path）、STOP/CONTINUE制御を実施。
- Lightweight DB (`tuff-db/src/lightweight`): 平文WAL + インメモリTag/Meaningで高速照合。WAL破損時は recovery mode に従って復旧。

## 主要メカニズム
- Physical Identity Protocol: AIのOrigin（起源）を固定し責任帰属を明確化。
- Identity Lock: 環境変数 `AI_ORIGIN` を参照（未指定時のデフォルトは `Gemini`）。本番運用や特定モデルでの検証時は、責任帰属を明確にするため明示的な指定を推奨する。内部的には `OnceLock` 等を用いて初期化を一度に限定し、プロセス実行中の再定義を物理的に防止する。
- Gap Resolver: 内部知識と外部事実の乖離を特定し、Transition（遷移）として編纂。
- Semantic Caching: 既知の事実をMIDで即答し、LLMの推論コストをスキップ。

## ロードマップ
- Phase 1: Foundation: ディレクトリ再配置、ワークスペース化、Originの動的固定実装。
- Phase 2: Connectivity: WebSocketプロトコル実装、アドオンのストリーミング送信対応。
- WebSocketプロトコル実装（仕様策定中。API型定義および `docs/protocol.md` は順次実装予定）。
- Phase 3: Verification: リアルタイム事実照合、ハルシネーション時のSTOP命令、WALへの遷移記録。
- Phase 4: Optimization: セマンティック・キャッシュによる低コスト化、プロバンス追跡による偏向拒絶。

## クイックスタート

### 実行コマンド
```bash
# ミドルウェア(MID)の起動
export AI_ORIGIN="Gemini" # 未指定時のデフォルトは "Gemini"
cargo run -p tuff_brg
```

### 環境変数
| 変数名 | 説明 | 例 |
| :--- | :--- | :--- |
| `AI_ORIGIN` | AIの自己識別子。Transitionに刻印される。 | `Gemini`, `GPT-4o` |
| `AGENT_ROLE` | 一時的な役割（任意）。 | `Verifier`, `Coder` |
| `OPENAI_API_KEY` | 検証用LLMのAPIキー。 | `sk-...` |
| `TUFF_FAST_PATH` | Lightweight Fast Path を有効化（`0`で無効）。 | `1` |
| `TUFF_LIGHTWEIGHT_MEANING_PATH` | `meaning.db` のパス。 | `_tuffdb/lightweight/meaning.db` |
| `TUFF_WAL_RECOVERY_MODE` | WAL復旧方針（`strict` または `truncate`）。 | `truncate` |

## meaning.db フォーマット
- 1行1エントリの `tag=meaning_snippet`。
- コメント行は `#` で開始。
- 例:

```text
prime-minister=Sanae Takaichi
shigeru-ishiba=Former Candidate
japan-leader=Takaichi
```

## Live Demo（ハルシネーション遮断）
```bash
# Anchor Facts を作成して実弾テスト
scripts/live_fire_test.sh
```

期待結果:
- `prime-minister\t...Shigeru Ishiba...` は `JudgeResult: SMOKE/GRAY_BLACK` + `ControlCommand: STOP`
- `prime-minister\t...Sanae Takaichi...` は `JudgeResult: WHITE/GRAY_WHITE`

## 商用利用
- 商用利用の方は `COMMERCIAL.md` をご確認ください。

## ディレクトリ構造と命名規則
本プロジェクトはワークスペース構成をとります。ディレクトリ名は `kebab-case` を採用しています。
- `tuff-db/`: コアエンジン
- `tuff-brg/`: ミドルウェア & アドオン

## 技術的背景と根拠
- 計算コストの激減: Semantic Caching により、検証済み事実を MID が即答することで LLM の推論ステップをスキップする（実装は `tuff-brg` 側で順次追加予定）。
- 知能自壊（モデル崩壊）の防止: `docs/architecture.md` を参照。

## 開発者向け注記
- ユニットテストは `cargo test -p tuff_db` で実行可能です（※現在、一部の基盤テストを順次拡充中のため、環境により未実装項目がある場合があります）。
