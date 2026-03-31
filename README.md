# precommit-ai

`precommit-ai` is a Rust CLI that analyzes your staged git diff with OpenAI, proposes a message, and then creates a commit after your confirmation.

## What It Does

1. Detects staged files in your current git repository.
2. Sends the staged diff to the OpenAI Chat Completions API.
3. Shows the generated output and asks for confirmation.
4. Prefixes your final commit with one of:
   - `feat`
   - `fix`
   - `nano`
   - `BREAKING-CHANGE`
5. Runs `git commit -m "<type>: <message>"`.

## Requirements

- Rust toolchain (`cargo`, `rustc`)
- Git
- `OPENAI_API_KEY` environment variable
- Staged changes (`git add ...`) in a git repo

## Installation

### Install from crates.io

```bash
cargo install precommit-ai
```

### Install from source

```bash
git clone https://github.com/etacassiopeia/precommit-ai.git
cd precommit-ai
cargo install --path .
```

## Setup

Create an OpenAI API key in the OpenAI dashboard, then export it:

```bash
export OPENAI_API_KEY="<your_api_key>"
```

To persist this, add it to your shell profile (`~/.zshrc`, `~/.bashrc`, etc.).

## Usage

Stage your changes first:

```bash
git add <files>
```

Run the CLI:

```bash
precommit-ai
```

You will be prompted to:

- Confirm the generated message
- Choose commit type (`feat`, `fix`, `nano`, `BREAKING-CHANGE`)

## Troubleshooting

- `OPENAI_API_KEY environment variable not found`
  - Export `OPENAI_API_KEY` before running the command.
- `No staged changes found. Make sure to stage your changes with git add.`
  - Run `git add ...` and retry.
- `Not a git repo`
  - Run inside a git working tree.
- `The diff is too large for the OpenAI API. Try reducing the number of staged changes, or write your own commit message.`
  - Commit in smaller chunks.

## Development

```bash
cargo check
cargo test
```

## Notes

- Current implementation uses the OpenAI Chat Completions endpoint.
- The tool is currently run manually as a CLI command.
