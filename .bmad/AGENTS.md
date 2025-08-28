# Solana Arbitrage Bot - Specialized Agents Documentation

This document defines the specialized Claude Code agents configured for the Solana Arbitrage Bot project. Each agent has specific expertise and tools optimized for different aspects of Solana DeFi development.

---

## 🎯 Project Context & Guidelines

### **Development Environment**
- **Language**: Rust (Cargo-based project)
- **Build System**: `cargo build --release --no-default-features --features monitor`
- **Testing**: `cargo test --no-default-features --features monitor --workspace`
- **Linting**: `cargo clippy --no-default-features --features monitor --all-targets -- -D warnings`
- **Formatting**: `cargo fmt --all -- --check`

### **Quality Gates**
- All code must pass `cargo clippy` with zero warnings
- Test coverage should maintain >90% for core modules
- All commits must pass CI/CD pipeline
- Security audits required before production deployment

### **Feature Flags**
- `monitor`: Core monitoring (production default)
- `full`: All features enabled (development)
- `web`: HTTP API and dashboard
- `sniper`: Token sniping functionality
- `gepa`: Genetic algorithm optimization

---

## 🦀 Rust Development Agents

### @rust-formatter
**Purpose**: Automatic Rust code formatting using cargo fmt  
**Tools**: Bash  
**Usage**: Immediate code formatting without confirmation  
**Command**: `cargo fmt --all`

### @rust-code-reviewer  
**Purpose**: Expert Rust code review and analysis using cargo clippy  
**Tools**: Bash  
**Usage**: Code quality review and best practices validation  
**Commands**: 
- `cargo clippy --all-targets -- -D warnings`
- `cargo clippy --no-default-features --features monitor --all-targets -- -D warnings`

### @rust-unit-test-generator
**Purpose**: Comprehensive unit test generation for Rust modules  
**Tools**: Edit, MultiEdit, Write, NotebookEdit, Glob, Grep, LS, Read, WebFetch, TodoWrite, WebSearch, BashOutput, KillBash  
**Usage**: Generate thorough test coverage for new functionality  
**Examples**:
- Create tests for `src/calculator.rs` profit calculations
- Generate safety tests for `src/safety.rs` circuit breakers
- Add integration tests for DEX monitoring

### @rust-refactoring-cto
**Purpose**: Large-scale code refactoring with safety guarantees  
**Tools**: Full toolkit (*)  
**Usage**: Major architectural changes while maintaining behavior  
**Examples**:
- Refactor transaction engine for better performance
- Migrate from synchronous to async patterns
- Restructure module organization for scalability

### @rust-bug-detective
**Purpose**: Systematic investigation of complex bugs in async Rust applications  
**Tools**: Full toolkit (*)  
**Usage**: Debug intermittent failures, race conditions, memory issues  
**Specialization**: Solana async applications with high-frequency trading

---

## ⚡ Solana Blockchain Agents

### @solana-architect
**Purpose**: Architectural guidance and code generation for Solana projects  
**Tools**: Glob, Grep, LS, Read, WebFetch, TodoWrite, WebSearch, BashOutput, KillBash, Edit, MultiEdit, Write, NotebookEdit, Bash  
**Usage**: Design decisions and technical consistency validation  
**Knowledge**: Solana SDK, program interactions, account management

### @solana-transaction-engine
**Purpose**: Solana transaction functionality implementation  
**Tools**: Glob, Grep, LS, Read, WebFetch, TodoWrite, WebSearch, BashOutput, KillBash, Edit, MultiEdit, Write, NotebookEdit, Bash  
**Usage**: Building transactions, lookup tables, priority fees, simulation  
**Specialization**: Jupiter API integration, atomic swaps, transaction optimization

### @solana-security-auditor
**Purpose**: Comprehensive security audit of Solana applications  
**Tools**: Full toolkit (*)  
**Usage**: Security review before production deployment  
**Focus Areas**: 
- Smart contract interaction safety
- Private key handling
- Economic exploit prevention
- MEV attack mitigation

---

## 🎯 Trading & MEV Specialists

### @jito-mev-implementer
**Purpose**: Jito protocol MEV functionality implementation  
**Tools**: Glob, Grep, LS, Read, WebFetch, TodoWrite, WebSearch, BashOutput, KillBash, Edit, MultiEdit, Write, NotebookEdit, Bash  
**Usage**: MEV protection and bundle transaction features  
**Expertise**: Jito bundles, private mempools, MEV protection strategies

### @onchain-event-monitor
**Purpose**: On-chain event monitoring and sniper bot functionality  
**Tools**: Glob, Grep, LS, Read, WebFetch, TodoWrite, WebSearch, BashOutput, KillBash, Edit, MultiEdit, Write, NotebookEdit, Bash  
**Usage**: Real-time event detection and automated trading responses  
**Specialization**: New pool detection, liquidity monitoring, sniper execution

---

## 🔧 Agent Usage Patterns

### Daily Development Workflow
```bash
# 1. Format code
@rust-formatter: cargo fmt --all

# 2. Review quality  
@rust-code-reviewer: cargo clippy analysis

# 3. Generate tests
@rust-unit-test-generator: Comprehensive test creation

# 4. Debug issues
@rust-bug-detective: Systematic problem investigation
```

### Feature Implementation
```bash
# 1. Architecture design
@solana-architect: Component structure planning

# 2. Core implementation  
@solana-transaction-engine: Transaction logic
@onchain-event-monitor: Event handling
@jito-mev-implementer: MEV protection

# 3. Quality assurance
@rust-unit-test-generator: Test coverage
@rust-code-reviewer: Code review
```

### Production Readiness
```bash
# 1. Security audit
@solana-security-auditor: Comprehensive security review

# 2. Performance optimization
@rust-refactoring-cto: Large-scale improvements

# 3. Final validation
@rust-code-reviewer: Quality confirmation
@rust-formatter: Code formatting
```

---

## 🎭 Agent Selection Guide

| Task Type | Primary Agent | Secondary Agent |
|-----------|---------------|-----------------|
| Code Formatting | @rust-formatter | - |
| Code Review | @rust-code-reviewer | @solana-security-auditor |
| Test Generation | @rust-unit-test-generator | - |
| Architecture Design | @solana-architect | @solana-security-auditor |
| Transaction Logic | @solana-transaction-engine | @solana-architect |
| MEV Protection | @jito-mev-implementer | @solana-security-auditor |
| Event Monitoring | @onchain-event-monitor | @solana-architect |
| Bug Investigation | @rust-bug-detective | @solana-architect |
| Major Refactoring | @rust-refactoring-cto | @solana-architect |
| Security Audit | @solana-security-auditor | @rust-code-reviewer |

---

## 🚀 Integration with BMAD Method

These specialized agents align with the BMAD Method workflow phases:

### Phase 1: Planning & Architecture
- **@solana-architect**: Technical feasibility analysis
- **@solana-security-auditor**: Security requirements assessment

### Phase 2: Story Implementation  
- **@onchain-event-monitor**: Event handling stories
- **@solana-transaction-engine**: Transaction execution stories
- **@jito-mev-implementer**: MEV protection stories

### Phase 3: Quality Assurance
- **@rust-unit-test-generator**: Test coverage stories
- **@rust-code-reviewer**: Code quality validation
- **@rust-formatter**: Code style enforcement

### Phase 4: Production Deployment
- **@solana-security-auditor**: Final security validation
- **@rust-refactoring-cto**: Performance optimizations
- **@rust-bug-detective**: Issue resolution

---

**Last Updated**: 2025-01-28  
**Project**: Solana Arbitrage Bot  
**Claude Code Agents**: 10 specialized agents configured