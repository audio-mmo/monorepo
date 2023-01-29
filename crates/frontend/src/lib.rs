use std::cell::RefCell;

use prost::Message;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use ammo_protos::frontend;

#[pyclass]
struct Client {
    client: ammo_client::Client,
}

thread_local! {
    static DECODING_BUFFER: RefCell<Vec<u8>> = RefCell::new(vec![]);
    static SERVICE_REQUEST_MSG: RefCell<frontend::ServiceRequestBatch> = RefCell::new(frontend::ServiceRequestBatch {
        requests: Vec::new(),
    });
}

fn encode_message<'p>(py: Python<'p>, msg: &impl Message) -> PyResult<&'p PyBytes> {
    let res = DECODING_BUFFER.with(|r| -> anyhow::Result<_> {
        let mut b = r.borrow_mut();
        b.clear();
        b.reserve(msg.encoded_len());
        msg.encode(&mut *b)?;
        Ok(PyBytes::new(py, &b[..]))
    })?;
    Ok(res)
}

#[pymethods]
impl Client {
    /// get the UI stack.
    ///
    /// Returns an empty stack if the underlying client doesn't yet have one for us to send.
    pub fn get_ui_stack<'a>(&self, py: Python<'a>) -> PyResult<&'a PyBytes> {
        let stack = self.client.get_ui_stack()?;
        let default_stack: frontend::UiStack = Default::default();
        let target_stack = if let Some(ref x) = stack {
            &*x
        } else {
            &default_stack
        };

        let res = DECODING_BUFFER.with(|buf_ref| -> anyhow::Result<_> {
            let mut buf = buf_ref.borrow_mut();
            let len = target_stack.encoded_len();
            buf.clear();
            buf.reserve(len);
            target_stack.encode(&mut *buf)?;
            Ok(PyBytes::new(py, &buf[..len]))
        })?;
        Ok(res)
    }

    pub fn dequeue_service_requests<'p>(&self, py: Python<'p>) -> PyResult<&'p PyBytes> {
        SERVICE_REQUEST_MSG.with(|r| {
            let mut msg = r.borrow_mut();
            msg.requests.clear();
            self.client.dequeue_service_requests(&mut msg.requests)?;
            encode_message(py, &*msg)
        })
    }

    pub fn ui_do_complete(&self, target: String, value: String) -> PyResult<()> {
        Ok(self.client.do_complete(target, value)?)
    }

    pub fn ui_do_cancel(&self, target: String) -> PyResult<()> {
        Ok(self.client.do_cancel(target)?)
    }
}

#[pyfunction]
fn start_client() -> PyResult<Client> {
    let client = ammo_client::Client::new()?;
    Ok(Client { client })
}

/// A Python module implemented in Rust.
#[pymodule]
fn ammo_frontend(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Client>()?;
    m.add_function(wrap_pyfunction!(start_client, m)?)?;
    Ok(())
}
