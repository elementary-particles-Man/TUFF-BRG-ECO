# TUFF-BRG WebSocket Protocol Specification (Draft)

## 目的
TUFF-BRG-Extension (Add-on) と TUFF-BRG (Middleware) 間の通信規約を固定し、
DOM断片のストリーミング検証と出力制御を一貫して実施する。

## 接続前提
- 初期フェーズは `localhost` 限定。
- 将来的なトークン認証を予約（`auth` メッセージ枠を保持）。

## メッセージ共通フォーマット
全メッセージは JSON。共通ヘッダを持つ。

```json
{
  "type": "StreamFragment | JudgeResult | ControlCommand | Auth",
  "id": "uuid",
  "ts": "RFC3339",
  "payload": { }
}
```

## メッセージ定義

### 1. StreamFragment (Add-on -> Middleware)
DOM断片の送信。

```json
{
  "type": "StreamFragment",
  "id": "...",
  "ts": "...",
  "payload": {
    "conversation_id": "...",
    "sequence_number": 0,
    "url": "https://example.com",
    "selector": "#chat",
    "fragment": "...DOM text...",
    "context": {
      "page_title": "...",
      "locale": "ja-JP"
    }
  }
}
```

### 2. JudgeResult (Middleware -> Add-on)
検証結果の返却。

```json
{
  "type": "JudgeResult",
  "id": "...",
  "ts": "...",
  "payload": {
    "status": "SMOKE | GRAY_BLACK | GRAY_MID | GRAY_WHITE | WHITE",
    "reason": "...",
    "confidence": 0.0,
    "claim": "...",
    "evidence_count": 0,
    "abstract_id": "uuid | null"
  }
}
```

### 3. ControlCommand (Middleware -> Add-on)
出力制御（STOP / CONTINUE）。

```json
{
  "type": "ControlCommand",
  "id": "...",
  "ts": "...",
  "payload": {
    "command": "STOP | CONTINUE",
    "trigger": "SmokeDetected | LowConfidence | ManualOverride",
    "detail": "..."
  }
}
```

### 4. Auth (Reserved)
将来の認証用。

```json
{
  "type": "Auth",
  "id": "...",
  "ts": "...",
  "payload": {
    "token": "..."
  }
}
```

## STOP 発火条件
- `VerificationStatus::Smoke` が確定した瞬間。
- `confidence` が閾値（暫定: 0.35）を下回った場合。

## STOP 後の復帰
- `ControlCommand: CONTINUE` 受信時は、Add-on は表示ブロックを解除する。
- `trigger: ManualOverride` の場合は、ユーザー操作による再開としてログに記録する。

## エラー処理
- JSON 解析失敗時は `ControlCommand: STOP` を返し、
  `trigger` を `ManualOverride` として通知。

## 認証・セキュリティ
- 初期段階は `localhost` のみ許可。
- 将来的に `Auth` メッセージでトークン検証を行う。

## 補足
- `confidence` の算出根拠（ログイット差分 / 証拠数等）は `docs/architecture.md` で定義する。
- `Auth` は接続直後に送信する方式を基本とし、将来的にメッセージ内トークンへ拡張可能とする。
