# My Rust OS

This is a simple Operating System (OS) written in Rust, created as a learning project by following an [online blog](https://os.phil-opp.com/). The goal of this project is to gain a deeper understanding of operating system concepts and low-level programming in Rust.

## Features

- Rust executable with no connection to the standard library
- 64-bit Rust kernel for the x86 architecture
- Interface for usage of VGA text mode
- CPU exceptions
- Double fault exceptions
- Hardware Interrupts
- Paging
- Heap Allocation

## Note
To try out the project, you need to first setup QEMU on your local machine.
I also recommend having [rustup](https://rustup.rs/) installed.
After that, you can:
1. Clone this repository with `git clone https://github.com/MarognaLorenzo/MarOS.git`
2. Run the project with `cargo run`
