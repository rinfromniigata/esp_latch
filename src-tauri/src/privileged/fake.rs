//! テスト用の`PrivilegedOps`。受け取った操作を記録し、決めておいた結果を順に返す。

use std::collections::VecDeque;
use std::sync::Mutex;

use super::{Op, PrivilegedError, PrivilegedOps, PrivilegedReply, Response};

#[derive(Default)]
pub struct FakePrivilegedOps {
    results: Mutex<VecDeque<Result<PrivilegedReply, PrivilegedError>>>,
    received: Mutex<Vec<Op>>,
    shutdowns: Mutex<usize>,
}

impl FakePrivilegedOps {
    /// 次の`execute`の結果を足す
    pub fn push(&self, result: Result<PrivilegedReply, PrivilegedError>) {
        self.results.lock().unwrap().push_back(result);
    }

    /// 次の`execute`で、起動なしにこの応答を返す
    pub fn push_response(&self, response: Response) {
        self.push(Ok(PrivilegedReply {
            launch: None,
            response,
        }));
    }

    /// 受け取った操作（受け取った順）
    pub fn received(&self) -> Vec<Op> {
        self.received.lock().unwrap().clone()
    }

    pub fn shutdowns(&self) -> usize {
        *self.shutdowns.lock().unwrap()
    }
}

impl PrivilegedOps for FakePrivilegedOps {
    fn execute(&self, op: Op) -> Result<PrivilegedReply, PrivilegedError> {
        self.received.lock().unwrap().push(op);
        self.results
            .lock()
            .unwrap()
            .pop_front()
            .expect("FakePrivilegedOps: no result prepared")
    }

    fn shutdown(&self) {
        *self.shutdowns.lock().unwrap() += 1;
    }
}
