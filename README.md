# bitcoinrs
Yet another bitcoin implementation in Rust

## Principles

**bitcoinrs** aims to achive
- [security](#secutiry)
- [portability](#portability)

but not
- high performance
- high memory efficiency


### Security

- Easy to use securely
  - Strongly typed
  - Intuitive data model
  - Pure data model
  - Simple
- Minimum `Mutex`
  - to avoid dead lock
- Non-unsafe
  - to avoid memory leak
- Minimum dependency
  - depend only a few trusted crates
- Well tested
- Well documented

### Portability

- Support a lot kind of OS and CPU
- Able to run on weak CPU
- Able to run on small Memory
- Flexibility
  - chose an implementation according as environment
