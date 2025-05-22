use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
pub enum ResourceStatus {
    Success(u32),
    Failed(String),
    QueuePosition(usize),
    Cancelled,
}

pub struct ResourcePool {
    request_tx: mpsc::UnboundedSender<ResourceRequest>,
    release_tx: mpsc::UnboundedSender<u32>,
    next_id: Arc<AtomicUsize>,
}

impl ResourcePool {
    pub fn new(resource_count: u32) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel::<ResourceRequest>();
        let (release_tx, release_rx) = mpsc::unbounded_channel::<u32>();
        let available_resources = VecDeque::from((0..resource_count).collect::<Vec<_>>());
        let pending_requests = VecDeque::new();
        let next_id = Arc::new(AtomicUsize::new(0));

        println!("Resource pool created with {} resources", resource_count);

        tokio::spawn(Self::resource_queue_manager(
            available_resources,
            pending_requests,
            request_rx,
            release_rx,
        ));

        Self {
            request_tx,
            release_tx,
            next_id,
        }
    }

    async fn resource_queue_manager(
        mut available_resources: VecDeque<u32>,
        mut pending_requests: VecDeque<ResourceRequest>,
        mut request_rx: mpsc::UnboundedReceiver<ResourceRequest>,
        mut release_rx: mpsc::UnboundedReceiver<u32>,
    ) {
        loop {
            println!("Resource queue manager loop");
            tokio::select! {
                Some(mut req) = request_rx.recv() => {
                    if let Some(res_id) = available_resources.pop_front() {
                        if req.cancel.try_recv().is_err() {
                            let _ = req.status.send(ResourceStatus::Success(res_id));
                        } else {
                            available_resources.push_back(res_id);
                            let _ = req.status.send(ResourceStatus::Cancelled);
                        }
                    } else {
                        let _id = req.id;
                        let _ = req.status.send(ResourceStatus::QueuePosition(pending_requests.len()));
                        pending_requests.push_back(req);
                    }
                },

                Some(res_id) = release_rx.recv() => {
                    while let Some(mut req) = pending_requests.pop_front() {
                        if req.cancel.try_recv().is_ok() {
                            let _ = req.status.send(ResourceStatus::Cancelled);
                            continue;
                        }
                        let _ = req.status.send(ResourceStatus::Success(res_id));
                        break;
                    }
                    if pending_requests.is_empty() {
                        available_resources.push_back(res_id);
                    }
                }
            }

            for (i, req) in pending_requests.iter().enumerate() {
                let _ = req.status.send(ResourceStatus::QueuePosition(i));
            }
        }
    }
}

#[derive(Clone)]
pub struct ResourceAllocator {
    request_tx: mpsc::UnboundedSender<ResourceRequest>,
    release_tx: mpsc::UnboundedSender<u32>,
    next_id: Arc<AtomicUsize>,
    cancel_tx: Arc<tokio::sync::Mutex<Option<oneshot::Sender<()>>>>,
}

impl ResourceAllocator {
    pub fn new(pool: &ResourcePool) -> Self {
        Self {
            request_tx: pool.request_tx.clone(),
            release_tx: pool.release_tx.clone(),
            next_id: Arc::clone(&pool.next_id),
            cancel_tx: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

        /// Sends a resource request and returns an UnboundedReceiver for status updates
    pub fn request_resource(&self) -> mpsc::UnboundedReceiver<ResourceStatus> {
        let (status_tx, status_rx) = mpsc::unbounded_channel();
        let cancel_tx = self.cancel_tx.clone();

        let req_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (res_tx, res_rx) = oneshot::channel();
        let (cancel_sender, cancel_receiver) = oneshot::channel();

        // Store cancel sender in the Mutex
        tokio::spawn({
            let cancel_tx_store = cancel_tx.clone();
            async move {
                let mut guard = cancel_tx_store.lock().await;
                *guard = Some(cancel_sender);
            }
        });

        let req = ResourceRequest {
            id: req_id,
            response: res_tx,
            cancel: cancel_receiver,
            status: status_tx.clone(),
        };

        // Send the request to the pool
        let _ = self.request_tx.send(req);

        // Spawn a task that listens for the oneshot result and sends failure if cancelled
        tokio::spawn(async move {
            match res_rx.await {
                Ok(_res_id) => {
                    // Successfully got resource, do nothing here
                }
                Err(_) => {
                    let _ = status_tx.send(ResourceStatus::Failed("Request cancelled".into()));
                }
            }
        });

        status_rx
    }

    pub fn release(&self, resource_id: u32) {
        println!("Releasing resource {}", resource_id);
        let _ = self.release_tx.send(resource_id);
    }

    pub async fn cancel(&self) {
        let mut guard = self.cancel_tx.lock().await;
        if let Some(cancel_sender) = guard.take() {
            let _ = cancel_sender.send(());
        }

        // Gotta fix this somehow so we have a way to notify a waiter that the request was cancelled
        // I'm tired and don't want to deal with it right now
        // let _ = status_tx.send(ResourceStatus::Cancelled);
    }
}

pub struct ResourceRequest {
    pub id: usize,
    pub response: oneshot::Sender<u32>,
    pub cancel: oneshot::Receiver<()>,
    pub status: mpsc::UnboundedSender<ResourceStatus>,
}
