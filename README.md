### Requirements

Before running the startup commands, ensure you have the following tools installed on your local machine:

1. Runtimes & SDKs

    Rust Toolchain: Required for the backend. Ensure you have rustc and cargo installed.

    please refer to the [official cargo documentation](https://doc.rust-lang.org/cargo/getting-started/installation.html).

    Node.js: Required for the frontend. Version 18.x or higher (LTS) is recommended.

    Yarn: The package manager used for the frontend (editor).

2. Command Line Tools

    just: A handy command runner used to trigger backend development tasks.

    Installation: brew install just (macOS) or cargo install just (via Rust).

    Please refer to the [official just documentation](https://github.com/casey/just?tab=readme-ov-file).

    Git: For version control and managing the repository.

3. First-Time Setup

    Before running the development servers for the first time, you must install the dependencies:

Frontend Dependencies:

```bash
cd frontend/editor
yarn install
```

### Startup Commands

```bash
# Start backend server

cd backend
just local-dev

# start frontend 

cd ../frontend/editor
yarn dev

```