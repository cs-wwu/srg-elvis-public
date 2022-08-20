# Systems Research Group - *Extensible Little Virtual Internet Simulation (Elvis)*

## Planning document

[https://docs.google.com/document/d/1HnhrICXHJyRhc0CUmUgXLJbSlxZfF9wzzvsMujmxk2A/edit](https://docs.google.com/document/d/1HnhrICXHJyRhc0CUmUgXLJbSlxZfF9wzzvsMujmxk2A/edit)

## Project Structure
The *Elvis* simulator itself is in the root level in the folder `sim`.

## Elvis Coding Conventions
*Elvis* is written primarily in Rust. These coding conventions apply to Rust code.

### Indentation
Standard indentation for Rust code shall be **4 spaces**.

### Comments
All code shall be accompanied with [rustdoc](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html)
comments. For example:
```
    /// Clone the buf. This clones the underlying ref counted buffer
    ///
    /// # Returns
    ///
    /// The new Buf that points to the same underlying ref counted data
    pub fn clone(&self) -> Buf {
        Buf {
            data: Rc::clone(&self.data),
            start: self.start,
            length: self.length
        }
    }
```

### Clippy
Rust has a standard linter that comes with Cargo. All code checked into the main branch
shall have a clean run of [clippy](https://github.com/rust-lang/rust-clippy).
That is, running `cargo clippy` should show no lint errors.

Even if your code runs without errors and all your tests pass, always still run clippy to
maintain high quality code style.

### Tests
Practice [Test Driven Development](https://en.wikipedia.org/wiki/Test-driven_development). 
All code checked into main should have a high coverage
for [unit tests](https://en.wikipedia.org/wiki/Unit_testing). Try not to check in code
that has no unit tests.

In a research project, true TDD is sometimes difficult because we are exploring new spaces.
To that end, it's ok to start with writing code. However, you should quickly follow up 
on your code with unit tests. Try and get as much coverage as possible, particularly
to public methods that you expose.

# Contact
*Elvis* administrators are [See-Mong Tan](mailto:see-mong.tan@wwu.edu), 
[Tim Harding](mailto:hardint4@wwu.edu) and [Robin Preble](mailto:prebler@wwu.edu).
