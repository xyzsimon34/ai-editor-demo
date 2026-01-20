# Yjs Agent Tools - Future Development Plan

**Status**: Planning  
**Purpose**: Design a MCP-style tool crate for agents to interact with Yjs documents

---

## Motivation

### Current Problem

Currently, agents (linter, composer, etc.) directly manipulate Yjs documents using low-level APIs:

- Agents must understand Yjs internal structure (XmlFragment, XmlElement, XmlText nodes)
- Direct document mutation makes it hard to provide suggestions to users
- No standardized interface for agents to read/modify documents
- Difficult to support Yjs documents with various formats (paragraphs, headings, lists, etc.)

### Goal

Create a **MCP (Model Context Protocol) style tool crate** that allows agents to:

1. **Read document content** via function calling
2. **Provide suggestions** to users (instead of direct mutations, like insert the ai suggested element)
3. **Operate on Yjs documents** without understanding low-level Yjs internals
4. **Support various Yjs formats** (paragraphs, headings, lists, etc.)

---

## Design Goals

### 1. MCP-Style Tool Interface

Agents should interact with Yjs through **function calling** (similar to OpenAI function calling):

```rust
// Example: Agent calls a tool to get document content
let content = yjs_tools::get_document_content(doc, fragment_name)?;

// Example: Agent provides a suggestion (not direct mutation)
let suggestion = yjs_tools::suggest_insertion(
    doc,
    position: "after paragraph 2",
    content: "New paragraph text"
)?;
```

### 2. Read Operations

**Primary Use Case**: Agent needs to read document content

```rust
// Tool: get_document_content
// Returns: Plain text or structured representation of the document
fn get_document_content(
    doc: &Arc<Doc>,
    fragment_name: &str,
) -> Result<DocumentContent>
```

**Questions to Resolve**:
- Should we return plain text or structured format (JSON with nodes)?
- How to represent different Yjs node types (paragraphs, headings, lists)?
- Should we include metadata (attributes, formatting)?

### 3. Suggestion Operations (Not Direct Mutations)

**Key Principle**: Agents provide **suggestions** to users, not direct modifications

```rust
// Tool: suggest_insertion
// Creates a suggestion that user can accept/reject
fn suggest_insertion(
    doc: &Arc<Doc>,
    position: InsertionPosition,
    content: &str,
) -> Result<SuggestionId>

// Tool: suggest_modification
fn suggest_modification(
    doc: &Arc<Doc>,
    target: NodeSelector,
    new_content: &str,
) -> Result<SuggestionId>

// Tool: suggest_deletion
fn suggest_deletion(
    doc: &Arc<Doc>,
    target: NodeSelector,
) -> Result<SuggestionId>
```

**Questions to Resolve**:
- How to represent suggestions? (separate Yjs structure, metadata, etc.)
- How do users accept/reject suggestions?
- Should suggestions be visible in the editor UI?

### 4. Node Selection & Positioning

Agents need to specify **where** to insert/modify content:

```rust
enum InsertionPosition {
    AfterNode(NodeSelector),
    BeforeNode(NodeSelector),
    AtIndex { fragment: String, index: usize },
    AtEnd { fragment: String },
}

enum NodeSelector {
    ByIndex { fragment: String, index: usize },
    ById { id: String },
    ByContent { fragment: String, text_match: String },
}
```

**Questions to Resolve**:
- How to uniquely identify nodes in Yjs? (IDs, indices, content matching?)
- What if document structure changes between suggestion and application?

### 5. Format Support

Yjs documents can have various formats:

- **Paragraphs**: `<paragraph>` elements with text nodes
- **Headings**: `<heading>` elements with level attributes
- **Lists**: `<list>` elements with `<list-item>` children
- **AI-generated content**: `<ai_generated>` elements with metadata

**Questions to Resolve**:
- Should tools be format-agnostic or format-aware?
- How to handle unknown/unsupported formats?
- Should we provide format-specific tools (e.g., `insert_paragraph`, `insert_heading`)?

---

## Proposed Architecture

### Crate Structure

```
yjs-agent-tools/
├── src/
    └── lib.rs
    
```

### TODO: Core Types


### TODO: Tool Functions (MCP-style)

---

## TODO: Integration with Agents

---

## Open Questions & Decisions Needed

### 1. Suggestion Storage

**Question**: How should suggestions be stored in Yjs?

**Options**:
- **Option A**: Separate Yjs structure (e.g., `suggestions` fragment)
  - Pros: Clean separation, easy to query
  - Cons: Need to sync with document structure
- **Option B**: Metadata on existing nodes
  - Pros: Suggestions tied to nodes
  - Cons: Clutters node attributes
- **Option C**: External storage (database)
  - Pros: Doesn't pollute Yjs structure
  - Cons: Requires persistence layer

**Team Input Required**: Which approach aligns with our architecture?

### 2. Suggestion Application

**Question**: How do users accept/reject suggestions?

**Options**:
- **Option A**: Frontend UI (buttons, keyboard shortcuts)
- **Option B**: WebSocket command from frontend
- **Option C**: REST API endpoint

**Team Input Required**: What's the expected UX flow?

### 3. Handling Concurrent Changes

**Questtion**: What happens if the document state changes after the Agent provides a suggestion but before the user reviews it?


**Options**:

- **Option A**: Format-Agnostic (Generic Tree/Text)

  Approach: Tools treat everything as generic Y.Xml or Y.Text nodes.

  Pros:
    - Future-proof: Supports new formats without updating the tool code.
    - Simplicity: One set of tools (read, insert, delete) fits all.

  Cons:
    - Agent Errors: The AI might generate invalid structures (e.g., a heading inside a list item).
    - Context Loss: The AI may not realize a block is a "Code Snippet" vs "Plain Text."

- **Option B**: Format-Aware (Semantic Schema)
Approach: Tools are strictly mapped to your schema (e.g., insert_heading, add_list_item).

  Pros:
    - Data Integrity: The system enforces valid document structures.
    - Precision: Agents make better decisions when they know they are editing a "Table" vs a "Paragraph."

  Cons:
    - Maintenance: Every schema change requires a tool update.
    - Rigidity: Harder to support experimental or third-party formats.

- **Option C**: Hybrid (Semantic Anchors + Agnostic Payloads)

  Approach: Use agnostic tools for location/navigation but provide a "Schema Context" hint to the Agent during the read phase.

  Pros:
    - Flexibility: The Agent understands it’s looking at a "Heading," but still uses a generic modify_node tool.
    - Balance: Best for rapid development while maintaining structural awareness.

  Cons: Requires a more sophisticated prompt and serialization logic.

### 4. Content Representation

**Question**: What format should `get_document_content` return?

**Options**:
- **Option A**: Plain text only
  - Pros: Simple, works for all agents
  - Cons: Loses structure information
- **Option B**: Structured JSON
  - Pros: Preserves structure, enables precise operations
  - Cons: More complex, format-dependent
- **Option C**: Both (plain text + optional structured)
  - Pros: Flexible, agents can choose
  - Cons: More implementation work

**Team Input Required**: What do agents actually need?

### 5. Node Identification

**Question**: How to uniquely identify nodes for suggestions?

**Options**:
- **Option A**: Index-based (fragment + index)
  - Pros: Simple, always available
  - Cons: Fragile if document changes
- **Option B**: ID-based (require nodes to have IDs)
  - Pros: Stable, survives structure changes
  - Cons: Need to add IDs to all nodes
- **Option C**: Content-based matching
  - Pros: Works with existing documents
  - Cons: Ambiguous if content repeats

**Team Input Required**: What's the reliability requirement?

### 6. Format Awareness

**Question**: Should tools be format-agnostic or format-aware?

**Options**:
- **Option A**: Format-agnostic (work with any Yjs structure)
  - Pros: Flexible, future-proof
  - Cons: Less helpful for agents
- **Option B**: Format-aware (understand paragraphs, headings, etc.)
  - Pros: Agents can make better suggestions
  - Cons: Need to maintain format definitions
- **Option C**: Hybrid (basic operations agnostic, advanced operations aware)
  - Pros: Best of both worlds
  - Cons: More complex API

**Team Input Required**: What formats do we need to support?

---

## Implementation Plan

### Phase 1: Read Operations (Week 1-2)

1. **Design `DocumentContent` type**
   - Plain text extraction
   - Optional structured format
   - Format detection

2. **Implement `get_document_content`**
   - Extract text from XmlFragment
   - Handle different node types
   - Return structured representation

3. **Testing**
   - Test with various document formats
   - Verify text extraction accuracy

### Phase 2: Basic Suggestions (Week 3-4)

1. **Design suggestion storage**
   - Decide on storage approach (see Open Questions)
   - Implement suggestion data structures

2. **Implement `suggest_insertion`**
   - Node selection/positioning
   - Suggestion creation
   - Suggestion storage

3. **Testing**
   - Test suggestion creation
   - Verify suggestion persistence

### Phase 3: Advanced Operations (Week 5-6)

1. **Implement `suggest_modification` and `suggest_deletion`**
2. **Implement node selectors**
3. **Add format-specific helpers**

### Phase 4: Agent Integration (Week 7-8)

1. **Refactor linter to use tools**
2. **Refactor composer to use tools**
3. **Update agent interfaces**

---

## Success Criteria

- [ ] Agents can read document content via function calling
- [ ] Agents can create suggestions (not direct mutations)
- [ ] Suggestions are visible/storable in Yjs structure
- [ ] Tools work with current document formats (paragraphs, etc.)
- [ ] Linter and composer successfully migrated to use tools
- [ ] New agents can easily use the tool crate

---

## Next Steps

1. **Team Review**: Review this document and answer open questions
2. **Design Decision**: Finalize suggestion storage approach
3. **Prototype**: Build minimal `get_document_content` to validate approach
4. **Implementation**: Start Phase 1 (Read Operations)

---

**Action Items**:
- [ ] Team review of design goals
- [ ] Answer all "Team Input Required" questions
- [ ] Decide on suggestion storage approach
- [ ] Create implementation tickets for Phase 1