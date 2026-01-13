# ✍️ Rust AI Editor Co-pilot 🤖

## 🚀 Project Overview

This project aims to provide a powerful backend AI automation collaboration system for editors. Through asynchronous processing and modular design, we can provide users with real-time content suggestions, grammar fixes, style optimization, and other automated editing features. The system is divided into two core modules: "Refiner" and "Intelligence (Agent)", each responsible for different AI tasks.

## 🎯 Core Design Principles

1.  **Layered Architecture**: Clear separation between HTTP interfaces, core business logic, and third-party AI services.
2.  **Asynchronous Processing**: Leverage Rust's `async/await` and `Future` to achieve high concurrency and high-performance request processing, avoiding I/O blocking.
3.  **Modularity & Extensibility**:
    * **Refiner**: Responsible for single, atomic text transformation tasks (expand, shorten, fix, optimize).
    * **Intelligence (Agent)**: Acts as the "brain", responsible for analyzing the current editor state and intelligently determining which Refiner or other Sub-agent should be triggered.
4.  **Clear Data Transfer Objects**: Use semantic Request/Response structures to improve API readability.
5.  **Lifetime Safety**: Make good use of Rust's ownership system, especially `&'static str` or `Arc<String>` to safely manage global resources (such as API keys).

## 📦 Module Structure

my-editor-project/
├── Cargo.toml                # Workspace configuration
├── api-server/               # Original bin/ content, now a separate crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # Axum entry point (original server.rs)
│       ├── handlers/         # HTTP Handlers (e.g., text_edit.rs)
│       ├── state.rs          # AppState definition (includes &'static str api_key)
│       └── model/            # API-level DTOs (RefineRequest, PulseResponse)
└── backend-core/             # Core business logic (Library)
    ├── Cargo.toml
    └── src/
        ├── lib.rs            # Public API exports
        ├── refiner/          # Text refinement module (single task)
        │   ├── mod.rs        
        │   ├── processor.rs  # call_fix_api, call_improve_api
        │   └── types.rs      # RefineInput, RefineOutput
        ├── intelligence/     # AI Agent module (routing & decision-making)
        │   ├── mod.rs        
        │   ├── brain.rs      # evaluate_pulse core logic
        │   └── types.rs      # Suggestion, PulseRequest
        └── errors.rs         # Core business error definitions (anyhow/thiserror)

## ⚙️ API Endpoints

### 1. Refiner API (Direct invocation of single refinement tasks)

* **POST `/refine/improve`**
    * **Purpose**: Optimize text quality and clarity.
    * **Request**: `Json<RefineRequest>`
    * **Response**: `Json<RefineResponse>`
* **POST `/refine/fix`**
    * **Purpose**: Fix grammar and spelling errors.
    * **Request**: `Json<RefineRequest>`
    * **Response**: `Json<RefineResponse>`
* **POST `/refine/longer`**
    * **Purpose**: Expand text length while maintaining the original meaning.
    * **Request**: `Json<RefineRequest>`
    * **Response**: `Json<RefineResponse>`
* **POST `/refine/shorter`**
    * **Purpose**: Shorten text length while maintaining the original meaning.
    * **Request**: `Json<RefineRequest>`
    * **Response**: `Json<RefineResponse>`

### 2. Intelligence API (Automated judgment and collaboration)

* **POST `/intelligence`**
    * **Purpose**: Frontend periodically sends editor state, backend AI evaluates and returns suggestions.
    * **Request**: `Json<PulseRequest>`
    * **Response**: `Json<PulseResponse>` (includes `Vec<Suggestion>`)

## ✨ Core Workflows

### 1. Refiner Workflow (Synchronous, user-initiated)

```mermaid
graph TD
    A[Frontend] --> B{POST /api/refine/<action>};
    B --> C{Axum Router};
    C --> D[handle_refine_request];
    D --> E{core::refiner::processor::call_xxx_api};
    E --> F[Third-party AI Service (e.g., OpenAI)];
    F --> E;
    E --> G{anyhow::Result<RefineOutput>};
    G --> D;
    D --> H{Json<RefineResponse>};
    H --> C;
    C --> A;
```

### 2. Intelligence (Agent) Workflow (Asynchronous, backend-initiated judgment)
