# AI Editor Backend

Collaborative editor backend in PoC stage, integrating Yjs CRDT synchronization with AI writing assistance features.

## 1. Quick Start

### Requirements

- Rust (edition 2024)
- Docker & Docker Compose
- `just` command tool

### Environment Variables

```bash
# Required
OPENAI_API_KEY=sk-...                    # OpenAI API key

# HTTP Server
BACKEND_HOST=0.0.0.0:3030                 # Listen address
BACKEND_CORS_ORIGINS=http://localhost:3000

# User Writing Detection
BACKEND_USER_WRITING_TIMEOUT_MS=2000     # Default 2 seconds
```

### Startup Commands

```bash
# Start full dev environment (PostgreSQL + Temporal + Backend)
just local

# Start HTTP server only (database must be running first)
just run

# Start worker and http server
just dev

# Build
just build
```

## 2. Protocols

### REST API

### Text Refinement Endpoints

All endpoints accept `RefineRequest` and return `RefineResponse`:

```rust
POST /improve    // Improve text quality
POST /fix        // Fix grammar and spelling
POST /longer     // Extend text length
POST /shorter    // Shorten text length
```

**RefineRequest**:
```json
{
  "text": "This is a test text."
}
```

**RefineResponse**:
```json
{
  "text": "This is an improved test text."
}
```

### WebSocket (`/ws`)

WebSocket uses a **dual-lane design**:

- **Binary**: Yjs state synchronization
- **Text**: AI commands and status notifications

#### Connection Flow

1. **On Connect**: Server immediately sends full document state (State Vector)
2. **Subscribe to Broadcast**: Clients automatically receive all Yjs updates and AI commands

#### Message Formats

**Binary (Yjs Update)**:
- Format: `Uint8Array` (Yjs v1 Update encoding)
- Source: Yrs Doc changes triggered by user input or AI modifications

**Text (AI Commands)**:
```json
// AI Status Notification
{
  "type": "AI_STATUS",
  "status": "thinking" | "complete" | "error",
  "message": "Polishing your text..."
}

// AI Result
{
  "type": "AI_RESULT",
  "status": "complete",
  "message": "Refined text content"
}

// Comment (Backseater)
{
  "type": "COMMENT",
  "comment_on": "specific text",
  "comment": "unhelpful comment",
  "color_hex": "#ff0000"
}
```

**Client Send (AI Command)**:
```json
{
  "type": "AI_COMMAND",
  "action": "IMPROVE" | "FIX" | "LONGER" | "SHORTER" | "AGENT" | "TOGGLE",
  "payload": {
    // Refiner: plain text string
    "text": "..."
    // Agent: role definition
    // { "role": "composer" }
  }
}
```

### Yjs/Yrs Synchronization Mechanism

**State Vector**:
- Uses `StateVector::default()` to encode full state on connection
- Clients automatically sync after applying updates

**Update Flow**:
1. User input → Frontend Yjs generates Update → WebSocket Binary send
2. Backend `doc.apply_update()` → Yrs Observer triggers
3. Observer broadcasts `MessageStructure::YjsUpdate` → All connected clients
4. AI modifications directly operate on `Arc<Doc>` → Also triggers Observer broadcast

**Key Implementation** (`http.rs:71-89`):
```rust
doc.observe_update_v1(move |_txn, update_event| {
    let update = update_event.update.to_vec();
    broadcast_tx.send(MessageStructure::YjsUpdate(update));
});
```

## 3. Architecture & Shortcuts

### Core Design

**Single Shared Document**: All users share the same `Arc<Doc>` instance, with all changes automatically broadcast via Yrs Observer. AI Agents directly operate on this Doc, and modifications are instantly synchronized to all connected clients.

**User Writing Detection**: Uses `AtomicU64` to track the last input timestamp. When AI appends word-by-word, it checks and interrupts to avoid conflicts with user input.


### TODO: migrate to persistence data storage
### PoC Limitations (Known Limitations)

#### Data Persistence
- ❌ **Single Document In-Memory**: All document content exists in `Arc<Doc>`, lost on restart
- ❌ **No Database Persistence**: Yjs updates are not written to PostgreSQL
- ❌ **No Multi-Document Support**: Currently only supports a single shared document

#### Authentication & Authorization
- ⚠️ **JWT Uses Fixtures**: Automatically uses test fixtures (`atb::fixtures::jwt`) when keys are not provided
- ❌ **No User Isolation**: All connections share the same document, no permission control

#### Error Handling
- ⚠️ **No WebSocket Reconnection**: Clients must implement their own reconnection logic
- ⚠️ **Simple AI API Failure Handling**: Only logs errors, no retry mechanism
- ⚠️ **Yjs Update Decode Failures**: Only logs warnings, does not disconnect

#### Feature Toggles
- ⚠️ **Global AtomicBool Flags**: `LINTER_FLAG`, `EMOJI_REPLACER_FLAG`, `BACKSEATER_FLAG` are global state, not per-session

#### Auto-Trigger Mechanism
- ⚠️ **Fixed 5-Second Cooldown**: Auto-checks (linter/emoji replacer/backseater) use hardcoded 5-second debounce
- ⚠️ **No Content Skip Logic**: Skips when document is empty or content unchanged, but lacks finer-grained condition checks

#### Performance Considerations
- ⚠️ **Broadcast Channel Capacity**: `broadcast::channel(100)` has fixed capacity, high-frequency updates may lose messages
- ⚠️ **AI Word-by-Word Append Delay**: `append_ai_content_word_by_word` uses fixed 100ms delay, no dynamic adjustment

## 4. Core Schema

### AppState

```rust
pub struct AppState {
    pub schema: AppSchema,                    // GraphQL Schema
    pub wf_engine: WorkflowEngine,            // Temporal workflow engine
    pub pg_pool: PgPool,                      // PostgreSQL connection pool
    pub jwt_encoder: Encoder,                  // JWT encoder
    pub jwt_decoder: Decoder,                  // JWT decoder
    pub api_key: String,                      // OpenAI API key
    pub editor_doc: Arc<Doc>,                  // Shared Yrs document
    pub editor_broadcast_tx: broadcast::Sender<MessageStructure>,  // Broadcast channel
    pub user_last_used_at: Arc<AtomicU64>,     // User last input timestamp
    pub user_writing_timeout_ms: u64,          // Writing timeout threshold (ms)
}
```

### MessageStructure

```rust
pub enum MessageStructure {
    YjsUpdate(Vec<u8>),      // Lane A: Yjs binary update
    AiCommand(String),        // Lane B: AI command JSON string
}
```

### AiCommand

```rust
pub struct AiCommand {
    pub r#type: String,                        // "AI_COMMAND"
    pub action: String,                        // "IMPROVE" | "FIX" | "LONGER" | "SHORTER" | "AGENT" | "TOGGLE"
    pub payload: Option<AiCommandPayload>,     // Command payload
}

pub enum AiCommandPayload {
    Refiner(String),          // Text refinement: plain text content
    Agent(AgentPayload),       // Agent: role definition
}

pub struct AgentPayload {
    pub role: String,          // Roles like "composer"
}
```

### REST API Models

```rust
pub struct RefineRequest {
    pub text: String,
}

pub struct RefineResponse {
    pub text: String,
}
```

### Yjs Document Structure

Document uses `XmlFragment` to store content:

- **Fragment Name**: `"content"`
- **Element Structure**: `<paragraph>` contains `<text>` nodes
- **AI Generated Marker**: `<ai_generated>` element with attribute `data-ai-generated="true"`

### Backseater Comments

```rust
pub struct BackseaterArgs {
    pub comment_on: String,    // Text fragment being commented on
    pub comment: String,       // Comment content
    pub color_hex: Option<String>,  // Color (hex)
}
```

---

**Note**: This document reflects PoC stage implementation. Some designs are simplified for rapid validation and require refactoring for production use.