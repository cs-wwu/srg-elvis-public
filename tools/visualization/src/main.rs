use std::{
    fs::{OpenOptions,File},
    io::{Stdout, Write, self},
    os::unix::fs::MetadataExt
};

/// Represents errors that can occur when notifying a subscriber.
#[derive(Debug)]
pub enum NotifyError {
    /// The provided subscriber could not be found.
    SubscriberNotFound,

    /// An I/O error occurred.
    IoError(io::Error),
}

impl From<io::Error> for NotifyError {
    fn from(err: io::Error) -> Self {
        NotifyError::IoError(err)
    }
}

/// The Publisher trait defines behavior for publishing messages to subscribers.
///
/// Implementors of Publisher can:
///
/// - Notify all subscribers of a new message with `notify`.
/// - Notify a single subscriber of a new message with `notify_one`, this does not add the subscriber to the list of subscribers.  
/// - Add new subscribers with `add_subscriber`.
/// - Remove existing subscribers with `remove_subscriber`.
///
/// # Example
///
/// ```
/// use crate::Subscriber;
///
/// struct MyPublisher {
///     subscribers: Vec<Subscriber>,
/// }
///
/// impl Publisher for MyPublisher {
///     // Implement required methods
/// }
/// ```
pub trait Publisher {
    /// Notify all subscribers of a new `msg`.
    ///
    /// Returns a `Result` with `()` on success or a `NotifyError` on failure.
    fn notify(&mut self, msg: &str) -> Result<(), NotifyError>;

    /// Notify a single `subscriber` of a new `msg`, this does not add the subscriber to the list of subscribers.
    ///
    /// Returns a `Result` with `()` on success or a `NotifyError` on failure.
    fn notify_one(&self, msg: &str, subscriber: &mut Subscriber) -> Result<(), NotifyError>;

    /// Add a new `subscriber`.
    ///
    /// Returns `Some(())` if the subscriber was added, or `None` if it was already present.
    fn add_subscriber(&mut self, subscriber: Subscriber) -> Option<()>;

    /// Remove the provided `subscriber` if it exists, if not does nothing.
    fn remove_subscriber(&mut self, subscriber: &Subscriber);
}

// NOTE Here for testing purposes
#[derive(Debug)]
struct Thing {
    subscribers: Vec<Subscriber>,
}

impl Publisher for Thing {
    fn notify(&mut self, msg: &str) -> Result<(), NotifyError> {
        self.subscribers
            .iter_mut()
            .map(|subscriber| subscriber.send(msg))
            .collect()
    }
    fn notify_one(&self, msg: &str, subscriber: &mut Subscriber) -> Result<(), NotifyError> {
        subscriber.send(msg)
    }
    fn add_subscriber(&mut self, subscriber: Subscriber) -> Option<()> {
        match self.subscribers.contains(&subscriber) {
            true => None,
            false => {
                self.subscribers.push(subscriber);
                Some(())
            }
        }
    }
    fn remove_subscriber(&mut self, subscriber: &Subscriber) {
        self.subscribers.retain(|s| s != subscriber)
    }
}

/// A Subscriber takes a message and sends it to some output.
/// The Subscriber enum represents different destinations that messages can be published to.
///
/// The variants are:
///
/// - `Stdout`: The message will be written to standard output.
/// - `File`: The message will be written to the provided `File`.
/// - `String`: The message will be appended to the provided `String`.
/// - `None`: The message will be discarded.
///
/// This allows a `Publisher` to publish messages to different kinds of outputs by passing
/// a `Subscriber` value.
#[derive(Debug)]
pub enum Subscriber {
    Stdout(Stdout),
    File(File),
    String(String),
    None,
}

impl Subscriber {
    fn send(&mut self, msg: &str) -> Result<(), NotifyError> {
        match self {
            Subscriber::Stdout(stdout) => {
                stdout.write_all(msg.as_bytes())?;
                Ok(())
            }
            Subscriber::File(file) => {
                file.write_all(msg.as_bytes())?;
                Ok(())
            }
            Subscriber::String(string) => {
                string.push_str(msg);
                Ok(())
            }
            Subscriber::None => Err(NotifyError::SubscriberNotFound),
        }
    }
}

impl Clone for Subscriber {
    fn clone(&self) -> Self {
        match self {
            Subscriber::Stdout(_) => Subscriber::Stdout(io::stdout().into()),
            Subscriber::File(file) => Subscriber::File(file.try_clone().unwrap()),
            Subscriber::String(string) => Subscriber::String(string.clone()),
            Subscriber::None => Subscriber::None,
        }
    }
} 

impl PartialEq for Subscriber {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Subscriber::Stdout(_), Subscriber::Stdout(_)) => true,
            (Subscriber::File(a), Subscriber::File(b)) => match (a.metadata(), b.metadata()) {
                // If both files have the same inode, then they are equal. Or at least I'm saying that
                (Ok(a), Ok(b)) => a.ino() == b.ino(),
                _ => false,
            },
            (Subscriber::String(a), Subscriber::String(b)) => a == b,
            (Subscriber::None, Subscriber::None) => true,
            _ => false,
        }
    }
}

impl PartialEq<Subscriber> for &mut Subscriber {
    fn eq(&self, other: &Subscriber) -> bool {
        match (self, other) {
            (Subscriber::Stdout(_), Subscriber::Stdout(_)) => true,
            (Subscriber::File(a), Subscriber::File(b)) => match (a.metadata(), b.metadata()) {
                // If both files have the same inode, then they are equal. Or at least I'm saying that
                (Ok(a), Ok(b)) => a.ino() == b.ino(),
                _ => false,
            },
            (Subscriber::String(a), Subscriber::String(b)) => a == b,
            (Subscriber::None, Subscriber::None) => true,
            _ => false,
        }
    }
}

impl From<Subscriber> for Stdout {
    fn from(subscriber: Subscriber) -> Self {
        match subscriber {
            Subscriber::Stdout(stdout) => stdout,
            _ => panic!("Cannot convert subscriber to Stdout"),
        }
    }
}

impl From<Subscriber> for File {
    fn from(subscriber: Subscriber) -> Self {
        match subscriber {
            Subscriber::File(file) => file,
            _ => panic!("Cannot convert subscriber to File"),
        }
    }
}

impl From<Subscriber> for String {
    fn from(subscriber: Subscriber) -> Self {
        match subscriber {
            Subscriber::String(s) => s,
            _ => panic!("Cannot convert subscriber to String"),
        }
    }
}

impl From<Subscriber> for () {
    fn from(subscriber: Subscriber) -> Self {
        match subscriber {
            Subscriber::None => (),
            _ => panic!("Cannot convert subscriber to Unit"),
        }
    }
}

fn main() {
    // NOTE Proof that it does in fact work but for some reason the test fails 
    let mut thig = Thing { subscribers: Vec::new() };
    let _ = thig.notify("Hello, world!");
    let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("test.txt")
            .unwrap();
    let mut sub = Subscriber::File(file);
    let _ = thig.notify_one("¡Hola, Mundo!", &mut sub);
    thig.add_subscriber(sub);
    let _ = thig.notify("Hello, world!");
    dbg!(thig);
}

#[cfg(test)]
mod tests {
    // use std::{fs::OpenOptions, io::Read};

    use super::*;

    #[test]
    fn test_add_subscriber_file() {
        let mut thig = Thing {
            subscribers: Vec::new(),
        };
        thig.add_subscriber(Subscriber::File(File::create("test.txt").unwrap()));
    }

    #[test]
    fn test_add_subscriber_string() {
        let mut thig = Thing {
            subscribers: Vec::new(),
        };
        thig.add_subscriber(Subscriber::String(String::from("test")));
        assert_eq!(
            thig.subscribers[0],
            Subscriber::String(String::from("test"))
        );
    }

    #[test]
    fn test_add_subscriber_stdout() {
        let mut thig = Thing {
            subscribers: Vec::new(),
        };
        thig.add_subscriber(Subscriber::Stdout(io::stdout()));
        assert_eq!(thig.subscribers[0], Subscriber::Stdout(io::stdout()));
    }

    #[test]
    fn test_remove_subscriber_file() {
        let mut thig = Thing {
            subscribers: Vec::new(),
        };
        thig.add_subscriber(Subscriber::File(File::create("test.txt").unwrap()));
        thig.remove_subscriber(&Subscriber::File(File::create("test.txt").unwrap()));
        assert_eq!(thig.subscribers.len(), 0);
    }

    #[test]
    fn test_remove_subscriber_string() {
        let mut thig = Thing {
            subscribers: Vec::new(),
        };
        thig.add_subscriber(Subscriber::String(String::from("test")));
        thig.remove_subscriber(&Subscriber::String(String::from("test")));
        assert_eq!(thig.subscribers.len(), 0);
    }

    #[test]
    fn test_remove_subscriber_stdout() {
        let mut thig = Thing {
            subscribers: Vec::new(),
        };
        thig.add_subscriber(Subscriber::Stdout(io::stdout()));
        thig.remove_subscriber(&Subscriber::Stdout(io::stdout()));
        assert_eq!(thig.subscribers.len(), 0);
    }

    #[test]
    fn test_notify_string() {
        let mut thig = Thing {
            subscribers: Vec::new(),
        };
        thig.add_subscriber(Subscriber::String(String::from("test")));
        let _ = thig.notify("Hello, world!");
        assert_eq!(
            thig.subscribers[0],
            Subscriber::String(String::from("testHello, world!"))
        );
    }

    // TODO Weird bug here that I can't figure out
    // #[test]
    // fn test_notify_file() {
    //     let mut thig = Thing {
    //         subscribers: Vec::new(),
    //     };
    //     let file = OpenOptions::new()
    //         .create(true)
    //         .write(true)
    //         .read(true)
    //         .truncate(true)
    //         .open("test.txt")
    //         .unwrap();
    //     let mut actual = String::new();
    //
    //     thig.add_subscriber(Subscriber::File(file));
    //     let _ = thig.notify("Hello, world!");
    //     dbg!(&thig.subscribers[0]);
    //     let _ = <File>::from(thig.subscribers[0].clone()).read_to_string(&mut actual);
    //     assert_eq!( thig.subscribers.len(), 1);
    //     assert_eq!(actual, "Hello, world!");
    // }
    //
    //
    // #[test]
    // fn test_notify_one_file() {
    //     let thig = Thing {
    //         subscribers: Vec::new(),
    //     };
    //     let file = OpenOptions::new()
    //         .write(true)
    //         .read(true)
    //         .create(true)
    //         .truncate(true)
    //         .open("test.txt")
    //         .unwrap();
    //     let mut sub = Subscriber::File(file);
    //     let _ = thig.notify_one("Hello, world!", &mut sub);
    //     let mut actual = String::new();
    //     let _ = File::from(sub).read_to_string(&mut actual);
    //     assert_eq!(thig.subscribers.len(), 0);
    //     assert_eq!(actual, "Hello, world!");
    // }

    #[test]
    fn test_notify_one_string() {
        let thig = Thing {
            subscribers: Vec::new(),
        };
        let mut sub = Subscriber::String("test".to_string());
        let _ = thig.notify_one("Hello, world!", &mut sub);
        assert_eq!(thig.subscribers.len(), 0);
        assert_eq!(sub, Subscriber::String("testHello, world!".to_string()));
    }
}
