//! Tests which aren't associated with a single component.
use std::borrow::Cow;

use proptest::prelude::*;

use crate::framer::*;
use crate::message::*;
use crate::parser::*;

/// Operations for our test, to get something approximating hypothesis rule-base testing.
#[derive(Clone, Debug, proptest_derive::Arbitrary)]
enum TestOps {
    /// Add a message to our queue of messages.
    Message {
        kind: MessageKind,
        identifier: MessageIdentifier,
        data: Vec<u8>,
    },

    /// Flush our queue of messages to the framer and ask the framer for all the bytes it has.
    FeedFramer,

    /// Feed all the bytes we've gotten from the framer to the parser in very small chunks, then drain the parser.
    FeedParser,
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn fuzz(mut ops: Vec<TestOps>) {
        let mut framer = Framer::new(1024);
        let mut parser = Parser::new(None, 1024);

        // We should always flush the framer and then the parser at least once.
        ops.extend([TestOps::FeedFramer, TestOps::FeedParser]);

        let mut expected_messages: Vec<Message> = vec![];
        let mut unfed_messages: Vec<Message> = vec![];
        let mut parsed_messages: Vec<Message> = vec![];
        let mut bytes: Vec<u8> = vec![];

        for o in ops  {
            match o {
                TestOps::Message{kind, identifier, data} => {
                    let m = Message::new(kind, identifier, Cow::Owned(data));
                    expected_messages.push(m.clone());
                    unfed_messages.push(m.clone());
                },
                TestOps::FeedFramer => {
                    for m in unfed_messages.iter() {
                        framer.add_message(m);
                    }
                    unfed_messages.clear();
                    bytes.extend(framer.get_data());
                    framer.clear();
                },
                TestOps::FeedParser => {
                    if bytes.is_empty() {
                        continue;
                    }
                    for mut chunk in (&bytes[..]).chunks(4) {
                        parser.feed(&mut chunk).expect("Should feed");
                        while let ParserOutcome::Message(m) = parser.read_message().expect("Should read") {
                            parsed_messages.push(m.clone_static());
                            parser.roll_forward().expect("Should roll");
                        }
                    }
                    bytes.clear();
                },
            }
        }

        assert_eq!(expected_messages, parsed_messages);
    }
}
