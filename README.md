# codeinput (ci)

`codeinput` is a command-line interface (CLI) tool designed for managing and analyzing `CODEOWNERS` files and other source code related tasks. It helps developers understand code ownership, track changes, and maintain codebase health.

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/CodeInputCorp/cli.git
   cd cli
   ```
2. Build the project:
   ```bash
   cargo build --release
   ```
   The executable `ci` will be available in `target/release/`. You may want to add this directory to your system's PATH or copy the executable to a directory in your PATH (e.g., `~/.local/bin` or `/usr/local/bin`).

Alternatively, if the project were published to crates.io, you could install it with:
```bash
# cargo install codeinput # Uncomment if published
```

## Usage

The `ci` tool provides several commands to interact with your codebase. Here are some of the main commands:

### General Commands
*   `ci --help`: Display help information and a list of all commands.
*   `ci <command> --help`: Display help for a specific command.
*   `ci config`: Show the current configuration being used by the CLI.

### CODEOWNERS Management
The `codeowners` subcommand provides tools for working with `CODEOWNERS` files:
*   `ci codeowners parse [--path <DIR_PATH>] [--cache-file <FILE_PATH>] [--format <json|bincode>]`: Parses `CODEOWNERS` files in the specified directory (default: current) and builds an ownership map, optionally caching it.
*   `ci codeowners list-files [--path <DIR_PATH>] [--tags <TAGS>] [--owners <OWNERS>] [--unowned] [--show-all] [--format <text|json|bincode>] [--cache-file <FILE_PATH>]`: Lists files and their owners, with various filtering options.
*   `ci codeowners list-owners [--path <DIR_PATH>] [--format <text|json|bincode>] [--cache-file <FILE_PATH>]`: Displays aggregated owner statistics.
*   `ci codeowners list-tags [--path <DIR_PATH>] [--format <text|json|bincode>] [--cache-file <FILE_PATH>]`: Analyzes and shows tag usage within `CODEOWNERS` files.

Example:
```bash
# Parse CODEOWNERS in the current directory and save to default cache
ci codeowners parse

# List all files owned by @username or team/name
ci codeowners list-files --owners "@username,team/name"

# List unowned files in JSON format
ci codeowners list-files --unowned --format json
```

### Shell Completion
You can generate shell completion scripts for `bash`, `zsh`, or `fish`:
*   `ci completion bash`: Generate bash completion script.
*   `ci completion zsh`: Generate zsh completion script.
*   `ci completion fish`: Generate fish completion script.

Example for bash:
```bash
ci completion bash > /etc/bash_completion.d/ci
# or source it in your .bashrc
# echo "source <(ci completion bash)" >> ~/.bashrc
```

### Global Options
*   `--config <FILE>`: Use a custom configuration file.
*   `--log-level <LEVEL>`: Set the logging level (e.g., `debug`, `info`, `warn`, `error`).
*   `--debug`: Enable debug mode (provides more verbose output).


## Contributing

We welcome contributions! Please see our [Contributing Guidelines](.github/CONTRIBUTING.md) for more details on how to get involved.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
