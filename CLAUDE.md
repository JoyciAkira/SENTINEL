# CLAUDE.md - AI Assistant Guide for Project Sentinel

## Project Overview

**Project Sentinel** is an AI coding agent with persistent goal alignment, designed to maintain absolute coherence from initial prompt to final outcome. It extends the Ralph Wiggum technique with hierarchical goal planning, alignment validation, intelligent auto-correction, and persistent memory for long-running projects.

### Core Vision

Sentinel addresses the fundamental problem of "goal drift" in AI coding agents by ensuring the agent never loses sight of the original objective, even across weeks of development.

**Key Innovation**: Unlike Cline/Kilocode or simple retry loops, Sentinel validates every action against the root goal and auto-corrects when deviations are detected.

---

## Architecture Overview

### High-Level Components

```
┌─────────────────────────────────────────────────────────────┐
│                     User Input (Prompt)                      │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    Goal Parser Module                        │
│  - Natural language → Structured ObjectiveTree              │
│  - Generates success criteria & validation tests            │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Objective Tree (State)                     │
│  - Hierarchical goal structure                              │
│  - Dependencies between goals                               │
│  - Progress tracking                                        │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                     Sentinel Loop                           │
│  ┌──────────────────────────────────────────────┐          │
│  │ 1. Select next task from tree                │          │
│  │ 2. Execute task (coding/testing/debugging)   │          │
│  │ 3. Validate alignment with root goal         │          │
│  │ 4. Auto-correct if deviation detected        │          │
│  │ 5. Update progress & persist state           │          │
│  │ 6. Loop until root goal complete             │          │
│  └──────────────────────────────────────────────┘          │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                  Alignment Validator                         │
│  - Validates code changes contribute to goal                │
│  - Detects deviations                                       │
│  - Suggests corrections                                     │
└─────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Language**: TypeScript (Node.js)
- **LLM Integration**: Claude 3.5 Sonnet (primary), with fallback support
- **Code Execution**: Docker containers for sandboxing
- **State Persistence**: SQLite for long-term storage, Redis for caching
- **Testing**: Playwright (E2E), Vitest (unit tests)
- **Base**: Fork of Cline/Kilocode codebase with Ralph Wiggum integration

---

## Codebase Structure (Planned)

```
sentinel/
├── src/
│   ├── goal-parser/              # Natural language → ObjectiveTree
│   │   ├── parser.ts             # LLM-powered parsing
│   │   ├── objective-tree.ts     # Tree data structure
│   │   └── success-criteria.ts   # Generate validation criteria
│   │
│   ├── sentinel/                 # Core execution loop
│   │   ├── loop.ts               # Main Sentinel loop
│   │   ├── task-selector.ts     # Choose next task from tree
│   │   └── executor.ts           # Execute tasks (wraps Ralph)
│   │
│   ├── alignment/                # Validation & correction
│   │   ├── validator.ts          # Check alignment with goals
│   │   ├── deviation-detector.ts # Identify off-track behavior
│   │   └── corrector.ts          # Generate correction plans
│   │
│   ├── context/                  # Long-term memory
│   │   ├── compressor.ts         # Compress execution history
│   │   ├── persistence.ts        # SQLite storage
│   │   └── retrieval.ts          # Embedding-based context search
│   │
│   ├── ralph-integration/        # Ralph Wiggum wrapper
│   │   ├── ralph-executor.ts    # Use Ralph for single tasks
│   │   └── ralph-config.ts       # Configuration
│   │
│   ├── execution/                # Code execution & testing
│   │   ├── sandbox.ts            # Docker container management
│   │   ├── test-runner.ts        # Run validation tests
│   │   └── code-generator.ts     # Generate code changes
│   │
│   └── ui/                       # User interface
│       ├── cli.ts                # Command-line interface
│       ├── progress-view.ts      # Real-time tree visualization
│       └── notifications.ts      # User notifications
│
├── tests/
│   ├── unit/                     # Unit tests
│   ├── integration/              # Integration tests
│   └── e2e/                      # End-to-end scenarios
│
├── docs/
│   ├── architecture.md           # Detailed architecture
│   ├── ralph-wiggum.md          # Ralph integration guide
│   └── examples/                 # Example projects
│
└── CLAUDE.md                     # This file
```

---

## Core Concepts

### 1. Ralph Wiggum Technique

**Original Concept**: Infinite loop that iterates until completion criteria are met.

```bash
/ralph-loop "Build feature X. Output <promise>DONE</promise>"
# Loops: Try → Fail → Analyze → Retry → Success
```

**Sentinel's Extension**: Uses Ralph as execution engine for individual tasks, but adds orchestration, validation, and multi-goal planning.

### 2. Objective Tree

Hierarchical structure representing project goals:

```typescript
interface Goal {
  id: string;
  description: string;
  successCriteria: string[];      // Verifiable criteria
  dependencies: string[];          // Prerequisite goal IDs
  status: 'pending' | 'in-progress' | 'completed' | 'blocked';
  validationTests: Test[];         // Automated tests
  estimatedComplexity: number;     // 1-10 scale
  parentId?: string;               // Parent goal
}

interface ObjectiveTree {
  rootGoal: Goal;
  subGoals: Goal[];
  currentFocus: string;            // Current task ID
  completionPercentage: number;
}
```

### 3. Alignment Validation

Every code change is validated against the root goal:

```typescript
interface AlignmentScore {
  isRelevant: boolean;             // Code is pertinent?
  contributesToGoal: number;       // 0-100 score
  deviations: string[];            // List of issues
  suggestedCorrections: Action[];  // Recommended fixes
}
```

**Validation Triggers**:
- After every file change
- After test execution
- At milestone completion
- Periodically during long tasks

### 4. Auto-Correction

When deviation detected (alignment score < 70):

1. **Analyze**: Why did we deviate?
2. **Plan**: Generate correction strategy
3. **Execute**: Apply correction
4. **Re-validate**: Check alignment improved

### 5. Persistent Memory

**Problem**: Context window limits prevent long-term project memory.

**Solution**: Compress and persist execution history:

```typescript
interface ProjectState {
  objectiveTree: ObjectiveTree;
  executionHistory: Action[];      // What agent did
  deviationHistory: Deviation[];   // When it went off-track
  learnings: Insight[];            // Patterns learned
  compressedContext: string;       // Summarized context
}
```

**Compression Strategy**:
- Keep: Decision rationale, key learnings, current state
- Discard: Verbose code, intermediate iterations
- Use: LLM summarization + embedding-based retrieval

---

## Development Workflows

### For AI Assistants Working on Sentinel

#### 1. Adding New Features

**Step 1**: Understand the goal hierarchy
```bash
# Read the objective tree structure
cat docs/objective-tree-spec.md
```

**Step 2**: Identify affected components
- Goal Parser: Does this change goal parsing?
- Sentinel Loop: Does this affect execution flow?
- Alignment: Does this change validation logic?
- Context: Does this affect memory management?

**Step 3**: Write tests first
```typescript
// tests/unit/feature-name.test.ts
describe('New Feature', () => {
  it('should maintain goal alignment', () => {
    // Test alignment validation
  });
});
```

**Step 4**: Implement with validation
- Write code
- Run alignment validation
- Ensure no goal drift introduced

**Step 5**: Integration testing
```bash
npm run test:integration
npm run test:e2e
```

#### 2. Debugging Issues

**Alignment Failure**: Check `logs/alignment-*.json`
- Review alignment scores over time
- Identify when deviation started
- Check correction attempts

**Execution Stuck**: Check `logs/sentinel-loop-*.json`
- Review task selection logic
- Check for circular dependencies
- Verify success criteria are achievable

**Memory Issues**: Check context compression
- Review `logs/context-compression-*.json`
- Verify important context not lost
- Check retrieval quality

#### 3. Testing Strategy

**Unit Tests**: Test individual components
```bash
npm run test:unit
```

**Integration Tests**: Test component interactions
```bash
npm run test:integration
```

**E2E Tests**: Complete workflows
```bash
npm run test:e2e -- simple-crud
npm run test:e2e -- medium-saas
npm run test:e2e -- complex-system
```

**Real-world Validation**:
- Test on actual projects (sandboxed)
- Measure goal completion accuracy
- Track alignment score trends

---

## Key Conventions

### 1. Code Style

- **TypeScript**: Strict mode enabled
- **Naming**:
  - PascalCase for classes/interfaces
  - camelCase for functions/variables
  - UPPER_SNAKE_CASE for constants
- **Files**: kebab-case.ts
- **Max line length**: 100 characters
- **Async/await**: Prefer over promises

### 2. Error Handling

```typescript
// Always use custom error types
class AlignmentError extends Error {
  constructor(
    message: string,
    public score: number,
    public deviations: string[]
  ) {
    super(message);
    this.name = 'AlignmentError';
  }
}

// Handle errors at appropriate level
try {
  const result = await executeTask(task);
} catch (error) {
  if (error instanceof AlignmentError) {
    // Auto-correct
    await correctDeviation(error);
  } else {
    // Escalate to user
    throw error;
  }
}
```

### 3. Logging

Use structured logging with context:

```typescript
logger.info('Task execution started', {
  taskId: task.id,
  goalId: task.goalId,
  timestamp: Date.now(),
  alignmentScore: 100
});

logger.warn('Alignment deviation detected', {
  taskId: task.id,
  alignmentScore: 65,
  deviations: ['hard-coded limit', 'missing error handling']
});
```

### 4. Testing

**Test Coverage Requirements**:
- Unit tests: >80% coverage
- Integration tests: All critical paths
- E2E tests: At least 3 complete scenarios

**Test Naming**:
```typescript
describe('AlignmentValidator', () => {
  describe('validateCodeAlignment', () => {
    it('should return high score for goal-aligned code', () => {});
    it('should detect hard-coded limits as deviation', () => {});
    it('should suggest corrections for deviations', () => {});
  });
});
```

### 5. Git Workflow

**Branches**:
- `main`: Stable releases
- `develop`: Integration branch
- `feature/feature-name`: New features
- `fix/bug-description`: Bug fixes

**Commits**:
```bash
# Format: <type>(<scope>): <description>
git commit -m "feat(alignment): add deviation detection algorithm"
git commit -m "fix(sentinel-loop): prevent infinite loops on circular deps"
git commit -m "docs(claude-md): update architecture diagrams"
```

**Types**: feat, fix, docs, test, refactor, perf, chore

---

## Implementation Roadmap

### Phase 1: MVP (4-6 weeks)

**Week 1-2: Foundation**
- [ ] Fork Cline/Kilocode codebase
- [ ] Setup project structure
- [ ] Implement basic ObjectiveTree data structure
- [ ] Create simple Goal Parser (hardcoded examples)

**Week 3-4: Core Loop**
- [ ] Implement Sentinel Loop (basic version)
- [ ] Add task selection logic
- [ ] Integrate Ralph Wiggum for execution
- [ ] Add basic progress tracking

**Week 5-6: Validation**
- [ ] Implement Alignment Validator
- [ ] Add success criteria checking
- [ ] Create simple CLI
- [ ] Test on simple projects (CRUD apps)

**Deliverable**: CLI tool that can complete simple single-feature projects with goal tracking.

### Phase 2: Auto-Correction (2-3 weeks)

**Week 1: Detection**
- [ ] Implement deviation detection
- [ ] Add alignment scoring
- [ ] Create deviation history tracking

**Week 2: Correction**
- [ ] Implement correction generator
- [ ] Add auto-correction execution
- [ ] Add re-validation loop

**Week 3: Testing**
- [ ] Test on medium projects (5-10 milestones)
- [ ] Measure correction success rate
- [ ] Refine correction strategies

**Deliverable**: System that auto-corrects when going off-track.

### Phase 3: Long-term Memory (2-3 weeks)

**Week 1: Compression**
- [ ] Implement context compression
- [ ] Add LLM-based summarization
- [ ] Test compression quality

**Week 2: Persistence**
- [ ] Add SQLite storage
- [ ] Implement state save/restore
- [ ] Add embedding-based retrieval

**Week 3: Testing**
- [ ] Test on long-running projects (1+ week)
- [ ] Verify context preservation
- [ ] Measure retrieval quality

**Deliverable**: System that maintains context across weeks.

### Phase 4: Advanced Features (Ongoing)

- [ ] Multi-agent collaboration
- [ ] Learning from past projects
- [ ] Custom validation rules
- [ ] VSCode extension
- [ ] Web dashboard
- [ ] Multi-language support (Python, Rust, Go)

---

## Working with This Codebase

### For AI Assistants (Claude, GPT-4, etc.)

**When asked to work on Sentinel, follow this process:**

1. **Understand Current State**
   - Read this CLAUDE.md file
   - Check current phase in roadmap
   - Review recent commits
   - Check open issues/todos

2. **Before Making Changes**
   - Identify affected components
   - Check if changes align with project vision
   - Verify no goal drift introduced
   - Write tests first

3. **During Development**
   - Follow code conventions
   - Add structured logging
   - Update documentation
   - Run tests frequently

4. **Before Committing**
   - Run full test suite
   - Update CLAUDE.md if architecture changed
   - Write clear commit message
   - Verify alignment with roadmap

5. **When Stuck**
   - Check logs/debugging section
   - Review similar implementations in codebase
   - Ask specific questions about blockers
   - Propose alternative approaches

### Common Pitfalls to Avoid

❌ **Don't**: Implement features not in roadmap without discussion
✅ **Do**: Propose features with alignment to vision

❌ **Don't**: Add complexity without clear benefit
✅ **Do**: Keep implementations simple and focused

❌ **Don't**: Skip tests
✅ **Do**: Write tests first, validate thoroughly

❌ **Don't**: Hardcode values
✅ **Do**: Use configuration with sensible defaults

❌ **Don't**: Ignore alignment validation
✅ **Do**: Validate every change contributes to goals

### Questions AI Assistants Should Ask

**Before starting work:**
- "What phase are we currently in?"
- "Are there any blockers or dependencies?"
- "What's the priority: speed or completeness?"

**During development:**
- "Should I implement minimal version first or full featured?"
- "Are there existing patterns I should follow?"
- "Do you want me to update tests/docs now or later?"

**When uncertain:**
- "I see two approaches (A: X, B: Y). Which aligns better with vision?"
- "This requires decision on [topic]. What's your preference?"
- "Should I proceed with current approach or explore alternatives?"

---

## Key Differentiators (Remember These!)

| vs Kilocode/Cline | vs Ralph Wiggum | Sentinel Advantage |
|-------------------|-----------------|-------------------|
| No goal tracking | Single goal only | Hierarchical objective tree |
| No validation | Blind retry | Alignment validation |
| Manual correction | Auto retry | Intelligent auto-correction |
| Short context | Git history only | Persistent memory |
| Single task | Single task | Multi-goal orchestration |

**Core Philosophy**:
> "Never lose sight of the root goal, no matter how many iterations or how complex the project."

---

## Resources

### Documentation
- `/docs/architecture.md` - Detailed technical architecture
- `/docs/ralph-wiggum.md` - Ralph Wiggum integration guide
- `/docs/examples/` - Example projects with walkthroughs

### External References
- [Ralph Wiggum Plugin](https://github.com/anthropics/claude-code/tree/main/plugins/ralph-wiggum)
- [Cline Repository](https://github.com/cline/cline)
- [Kilocode Repository](https://github.com/kilocode/kilocode)

### Community
- GitHub Issues: [github.com/JoyciAkira/SENTINEL/issues]
- Discussions: [github.com/JoyciAkira/SENTINEL/discussions]

---

## Quick Reference

### Common Commands (Planned)

```bash
# Initialize new project
sentinel init "Build feature X with requirements Y"

# Resume existing project
sentinel resume ./project-path

# Check progress
sentinel status

# Modify goals
sentinel modify-goal "Add requirement Z"

# Pause/resume
sentinel pause
sentinel resume

# View alignment history
sentinel alignment-history

# Debug issues
sentinel debug --logs
sentinel debug --alignment
sentinel debug --context
```

### Key Files to Check

- **Current state**: `.sentinel/state.json`
- **Objective tree**: `.sentinel/objective-tree.json`
- **Alignment history**: `.sentinel/logs/alignment-*.json`
- **Execution logs**: `.sentinel/logs/execution-*.json`
- **Compressed context**: `.sentinel/context/compressed-*.json`

---

## Version History

- **v0.1.0** (Current): Initial CLAUDE.md creation, repository setup
- **v0.2.0** (Planned): MVP implementation
- **v0.3.0** (Planned): Auto-correction system
- **v0.4.0** (Planned): Long-term memory
- **v1.0.0** (Planned): Production-ready release

---

## Contributing

**For AI Assistants contributing to Sentinel:**

1. Read this entire CLAUDE.md file
2. Understand the vision and architecture
3. Check current roadmap phase
4. Follow conventions strictly
5. Write tests for everything
6. Maintain goal alignment in all changes
7. Update documentation with changes
8. Ask clarifying questions when uncertain

**Remember**: Sentinel is meta - it's an AI agent that prevents goal drift in AI coding. When working on Sentinel, we must practice what we preach: maintain alignment with the vision at all times.

---

## Contact & Support

- **Repository**: github.com/JoyciAkira/SENTINEL
- **Issues**: github.com/JoyciAkira/SENTINEL/issues
- **Discussions**: github.com/JoyciAkira/SENTINEL/discussions

---

**Last Updated**: 2026-01-23
**Maintained By**: Project Sentinel Team
**For**: AI Assistants working on autonomous coding agents

---

*"Never lose sight of the goal."* - Sentinel Project Motto
