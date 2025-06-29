

# Git Commit Hash Modifier

This program modifies Git commits by appending a nonce to the committer name and searching for specific hash properties, such as a desired hash prefix or a hidden message. It uses multithreading to speed up the search process.

## Features

- Search for a Git commit hash that starts with a specific prefix.
- Search for a Git commit hash that contains a hidden message.
- Multithreaded execution for faster processing.
- Adjustable starting nonce and thread count.

## Requirements

- Rust (stable version)
- A Git repository (the program operates on the current repository)

## Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd <repository-folder>
   ```

2. Build the program in release mode:
   ```bash
   cargo build --release
   ```

3. The compiled binary will be located in the `target/release` directory.

## Usage

Run the program from the root of a Git repository. The program modifies the most recent commit.

### Basic Syntax

```bash
cargo run --release -- [OPTIONS]
```
or using the executable directly assuming you have added it to your path:

```bash
git-custom-hash [OPTIONS]
```

### Options

| Option         | Description                                                                 |
|-----------------|-----------------------------------------------------------------------------|
| `-h <prefix>`  | Specify the desired hash prefix (e.g., `-h 000000`).                        |
| `-m <message>` | Specify a hidden message to search for in the hash (e.g., `-m deadbeef`).   |
| `-n <nonce>`   | Specify the starting nonce as a hexadecimal value (default: `1`).           |
| `-j <threads>` | Specify the number of threads to use (default: number of CPU cores).        |

### Examples

#### Search for a Hash Starting with a Prefix

```bash
cargo run --release -- -h 000000
```

This command searches for a commit hash that starts with `000000`.

#### Search for a Hash Containing a Hidden Message

```bash
cargo run --release -- -m deadbeef
```

This command searches for a commit hash that contains the hidden message `deadbeef`.

#### Specify a Starting Nonce

```bash
cargo run --release -- -h 000000 -n 1A
```

This command starts the search with a hexadecimal nonce of `0x1A` (decimal `26`).  This is useful in cases where the previous search was halted early, but the last used nonce is known (command line output)

#### Use a Custom Number of Threads

```bash
cargo run --release -- -h 000000 -j 4
```

This command uses 4 threads for the search.

#### Combine Options

```bash
cargo run --release -- -h 000000 -m deadbeef -n 100 -j 8
```

This command searches for a hash that starts with `000000` and contains the hidden message `deadbeef`, starting with a nonce of `0x100` (decimal `256`) and using 8 threads.

## Output

The program outputs the following information during execution:

1. **Hashes per Second**: The number of hashes being tested per second, displayed every 5 seconds.
2. **Most Recent Nonce**: The most recently tested nonce value in hexadecimal format.
3. **Result**: If a matching hash is found, the program outputs the nonce that produced it.

Example output:
```
Searching for hash starting with: 000000
Starting nonce: 1
Using 8 threads.
Hashes per second: 50K	Most recent nonce: 1F4
Hashes per second: 52K	Most recent nonce: 3E8
A thread found: 0000001A
```

## Notes

- The program modifies the most recent commit in the current Git repository. Ensure you have a backup or are working in a safe environment before running the program.
- If no matching hash is found, the program will terminate without making changes to the repository.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.
