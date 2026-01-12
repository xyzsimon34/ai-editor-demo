---
description: AI Text Editor Demo Project Rules & Context
globs: frontend/editor/**/*.{ts,tsx}
---

# AI Text Editor Demo Project Standards

You are an expert full-stack developer specializing in **Next.js 15**, **TipTap**, **Novel**, and **TailwindCSS**. You are assisting in building a Notion-style AI editor.

## Project Architecture

- **Framework**: Next.js 15 (App Router)
- **Editor Core**: TipTap / ProseMirror via the **Novel** package.
- **AI Integration**: Vercel AI SDK with OpenAI.
- **Styling**: TailwindCSS + Radix UI.

## File Structure Conventions

- `src/app/api/`: All AI streaming and backend logic.
- `src/components/generative/`: Components related to AI prompts and AI-driven UI.
- `src/components/selectors/`: Popover menus for text formatting (Color, Bubble menu).
- `src/lib/`: Shared utility functions.

## Technical Requirements

### 1. Editor Extensions
- When adding new functionality, register it in `src/components/extensions.ts`.
- Follow the **Novel** pattern for extension configuration.

### 2. AI Implementation
- Use the **Vercel AI SDK** (`useCompletion` or `useChat`).
- Ensure all AI responses are streamed for a better UX.
- Custom prompts should be modularized within the `generative` directory.

### 3. State & Storage
- Priority: Local editor state -> Debounced `localStorage` persistence.
- Do not trigger heavy re-renders on every keystroke.

### 4. Code Style
- Use **TypeScript** for all files; define interfaces for props.
- Use functional components with `lucide-react` for icons.
- Follow Tailwind CSS class sorting (use `cn()` utility for conditional classes).

## Common Tasks

- **Adding a Slash Command**: Modify the `Command` extension in the editor setup.
- **Updating AI Prompt**: Check `src/app/api/generate/route.ts` for the system prompt logic.
- **UI Tweaks**: Use Radix UI primitives to maintain accessibility.

## Important Constraints
- **Do not** replace TipTap extensions with standard HTML elements.
- **Do not** remove the `novel` tailwind classes, as they handle the specific Notion-like typography.
- Ensure `use client` directive is present for all interactive editor components.