//! ✅ IntelliJ Pro: Asynchronous Syntax Highlighting Job Queue
//! Separates heavy syntax analysis from UI thread for zero-latency input

use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;

/// ✅ IntelliJ Pro: Syntax highlight job
#[derive(Clone, Debug)]
pub struct HighlightJob {
    pub line_idx: usize,
    pub text: String,
    pub version: u64,  // Buffer version when job was created
}

/// ✅ IntelliJ Pro: Job queue for async syntax highlighting
pub struct HighlightJobQueue {
    jobs: Rc<RefCell<VecDeque<HighlightJob>>>,
    max_queue_size: usize,
}

impl HighlightJobQueue {
    /// Create a new job queue
    pub fn new() -> Self {
        Self {
            jobs: Rc::new(RefCell::new(VecDeque::new())),
            max_queue_size: 100,  // Prevent queue from growing too large
        }
    }

    /// ✅ IntelliJ Pro: Enqueue a highlight job (batching)
    /// If queue is full, only keep the most recent jobs
    pub fn enqueue(&self, job: HighlightJob) {
        let mut jobs = self.jobs.borrow_mut();
        
        // Remove duplicate jobs for the same line (keep only latest)
        jobs.retain(|j| j.line_idx != job.line_idx);
        
        jobs.push_back(job);
        
        // Limit queue size to prevent memory explosion
        while jobs.len() > self.max_queue_size {
            jobs.pop_front();
        }
    }

    /// ✅ IntelliJ Pro: Dequeue a job for processing
    pub fn dequeue(&self) -> Option<HighlightJob> {
        self.jobs.borrow_mut().pop_front()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.jobs.borrow().is_empty()
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.jobs.borrow().len()
    }

    /// Clear all pending jobs (on large edits)
    pub fn clear(&self) {
        self.jobs.borrow_mut().clear();
    }

    /// ✅ IntelliJ Pro: Batch enqueue multiple jobs (for visible range)
    pub fn enqueue_batch(&self, jobs: Vec<HighlightJob>) {
        for job in jobs {
            self.enqueue(job);
        }
    }
}

impl Default for HighlightJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_basic() {
        let queue = HighlightJobQueue::new();
        
        assert!(queue.is_empty());
        
        queue.enqueue(HighlightJob {
            line_idx: 0,
            text: "test".to_string(),
            version: 1,
        });
        
        assert_eq!(queue.len(), 1);
        
        let job = queue.dequeue();
        assert!(job.is_some());
        assert_eq!(job.unwrap().line_idx, 0);
        
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_deduplication() {
        let queue = HighlightJobQueue::new();
        
        // Enqueue same line twice
        queue.enqueue(HighlightJob {
            line_idx: 5,
            text: "old".to_string(),
            version: 1,
        });
        
        queue.enqueue(HighlightJob {
            line_idx: 5,
            text: "new".to_string(),
            version: 2,
        });
        
        // Should only have 1 job (latest one)
        assert_eq!(queue.len(), 1);
        
        let job = queue.dequeue().unwrap();
        assert_eq!(job.text, "new");
        assert_eq!(job.version, 2);
    }

    #[test]
    fn test_queue_max_size() {
        let queue = HighlightJobQueue::new();
        
        // Enqueue 150 jobs (exceeds max of 100)
        for i in 0..150 {
            queue.enqueue(HighlightJob {
                line_idx: i,
                text: format!("line {}", i),
                version: 1,
            });
        }
        
        // Should be capped at 100
        assert_eq!(queue.len(), 100);
        
        // First job should be from line 50 (first 50 were dropped)
        let first_job = queue.dequeue().unwrap();
        assert_eq!(first_job.line_idx, 50);
    }
}
