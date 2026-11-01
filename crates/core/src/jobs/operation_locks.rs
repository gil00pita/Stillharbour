//! Process-local atomic admission for overlapping native directory scopes.
//! This coordinates owned jobs; it is not filesystem authorization or an OS lock.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::DirectoryChain,
};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, OnceLock},
};

const MAX_LEASES: usize = 64;
const MAX_SCOPES: usize = 64;
type Object = (u64, u64);
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}
pub(crate) struct Scope {
    directory: DirectoryChain,
    access: Access,
}
impl Scope {
    pub(crate) fn new(directory: DirectoryChain, access: Access) -> Self {
        Self { directory, access }
    }
}
#[derive(Clone)]
struct Key {
    ancestors: Vec<Object>,
    access: Access,
}
impl Key {
    fn conflicts(&self, other: &Self) -> bool {
        (self.access == Access::Write || other.access == Access::Write)
            && (self
                .ancestors
                .contains(other.ancestors.last().expect("validated scope"))
                || other
                    .ancestors
                    .contains(self.ancestors.last().expect("validated scope")))
    }
}
#[derive(Default)]
struct State {
    next: u64,
    active: Vec<(u64, Vec<Key>)>,
}
#[derive(Default)]
pub(crate) struct Registry {
    state: Mutex<State>,
}
pub(crate) struct Lease {
    registry: Arc<Registry>,
    id: u64,
    _scopes: Vec<Scope>,
}
fn busy() -> AppError {
    AppError::new(
        ErrorCode::Busy,
        "Another job holds an incompatible library or destination scope. Retry after it finishes.",
    )
}
impl Registry {
    pub(crate) fn shared() -> Arc<Self> {
        static SHARED: OnceLock<Arc<Registry>> = OnceLock::new();
        SHARED.get_or_init(|| Arc::new(Self::default())).clone()
    }
    pub(crate) fn try_acquire(self: &Arc<Self>, scopes: Vec<Scope>) -> Result<Lease> {
        if scopes.is_empty() || scopes.len() > MAX_SCOPES {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Lock between 1 and 64 native directory scopes together.",
            ));
        }
        let mut keys = Vec::with_capacity(scopes.len());
        for scope in &scopes {
            let ancestors: Vec<_> = scope
                .directory
                .identities()
                .iter()
                .map(|id| (id.volume, id.file))
                .collect();
            if ancestors.is_empty()
                || ancestors.len() > 96
                || ancestors.iter().copied().collect::<BTreeSet<_>>().len() != ancestors.len()
            {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Invalid native directory ancestry for operation admission.",
                ));
            }
            keys.push(Key {
                ancestors,
                access: scope.access,
            });
        }
        let mut state = self.state.lock().map_err(|_| busy())?;
        if state.active.len() >= MAX_LEASES
            || state.active.iter().any(|(_, held)| {
                held.iter()
                    .any(|left| keys.iter().any(|right| left.conflicts(right)))
            })
        {
            return Err(busy());
        }
        let id = state.next.checked_add(1).ok_or_else(busy)?;
        // Check the entire request before inserting anything: no partial holds
        // or lock-order deadlocks when two jobs request roots in opposite order.
        state.active.push((id, keys));
        state.next = id;
        Ok(Lease {
            registry: self.clone(),
            id,
            _scopes: scopes,
        })
    }
}
impl Lease {
    #[cfg(any(test, feature = "fixture-probes", feature = "copy-workflow"))]
    pub(crate) fn directory(&self, index: usize) -> &DirectoryChain {
        &self
            ._scopes
            .get(index)
            .expect("backend-owned lock scope index")
            .directory
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        // Releasing a known lease remains safe after poison; new admission
        // stays fail-closed until restart if an earlier owner panicked in state.
        let mut state = self
            .registry
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.active.retain(|(id, _)| *id != self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let run = tempfile::tempdir().unwrap();
        let root = run.path().canonicalize().unwrap();
        let a = root.join("a");
        let b = root.join("b");
        let child = a.join("child");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::create_dir(&b).unwrap();
        std::fs::write(a.join("preserved.bin"), b"retained source").unwrap();
        (run, a, b, child)
    }
    fn scope(path: &Path, access: Access) -> Scope {
        Scope::new(DirectoryChain::open(path).unwrap(), access)
    }
    #[test]
    fn ancestor_and_descendant_writes_conflict_but_readers_and_siblings_coexist() {
        let (_run, a, b, child) = fixture();
        let registry = Arc::new(Registry::default());
        let read = registry.try_acquire(vec![scope(&a, Access::Read)]).unwrap();
        let read_child = registry
            .try_acquire(vec![scope(&child, Access::Read)])
            .unwrap();
        assert!(
            registry
                .try_acquire(vec![scope(&child, Access::Write)])
                .is_err()
        );
        assert!(
            registry
                .try_acquire(vec![scope(&a, Access::Write)])
                .is_err()
        );
        let sibling = registry
            .try_acquire(vec![scope(&b, Access::Write)])
            .unwrap();
        drop((read, read_child, sibling));
        let write = registry
            .try_acquire(vec![scope(&child, Access::Write)])
            .unwrap();
        assert!(registry.try_acquire(vec![scope(&a, Access::Read)]).is_err());
        assert!(
            registry
                .try_acquire(vec![scope(&a, Access::Write)])
                .is_err()
        );
        drop(write);
        assert_eq!(
            std::fs::read(a.join("preserved.bin")).unwrap(),
            b"retained source"
        );
    }
    #[test]
    fn multi_scope_conflict_retains_no_partial_hold_and_drop_releases_the_whole_lease() {
        let (_run, a, b, _) = fixture();
        let registry = Arc::new(Registry::default());
        let first = registry
            .try_acquire(vec![scope(&a, Access::Write)])
            .unwrap();
        assert!(
            registry
                .try_acquire(vec![scope(&b, Access::Write), scope(&a, Access::Read)])
                .is_err()
        );
        let other = registry
            .try_acquire(vec![scope(&b, Access::Write)])
            .unwrap();
        drop((first, other));
        let pair = registry
            .try_acquire(vec![scope(&b, Access::Write), scope(&a, Access::Write)])
            .unwrap();
        assert!(registry.try_acquire(vec![scope(&a, Access::Read)]).is_err());
        assert!(registry.try_acquire(vec![scope(&b, Access::Read)]).is_err());
        drop(pair);
        assert!(
            registry
                .try_acquire(vec![scope(&a, Access::Write), scope(&b, Access::Write)])
                .is_ok()
        );
    }
    #[test]
    fn concurrent_opposite_order_requests_admit_exactly_one_whole_scope() {
        let (_run, a, b, _) = fixture();
        let registry = Arc::new(Registry::default());
        let start = Arc::new(std::sync::Barrier::new(3));
        let finish = start.clone();
        let (send, receive) = std::sync::mpsc::channel();
        let mut threads = Vec::new();
        for paths in [[a.clone(), b.clone()], [b, a]] {
            let registry = registry.clone();
            let barrier = start.clone();
            let send = send.clone();
            threads.push(std::thread::spawn(move || {
                let scopes = paths
                    .iter()
                    .map(|path| scope(path, Access::Write))
                    .collect();
                barrier.wait();
                let lease = registry.try_acquire(scopes);
                send.send(lease.is_ok()).unwrap();
                barrier.wait();
                drop(lease);
            }));
        }
        finish.wait();
        let admitted = [receive.recv().unwrap(), receive.recv().unwrap()];
        assert_eq!(admitted.iter().filter(|admitted| **admitted).count(), 1);
        finish.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(registry.state.lock().unwrap().active.is_empty());
    }
    #[test]
    fn bounded_admission_and_invalid_empty_scopes_never_expand_the_registry() {
        let (_run, a, _, _) = fixture();
        let registry = Arc::new(Registry::default());
        assert!(registry.try_acquire(vec![]).is_err());
        let held: Vec<_> = (0..MAX_LEASES)
            .map(|_| registry.try_acquire(vec![scope(&a, Access::Read)]).unwrap())
            .collect();
        assert!(registry.try_acquire(vec![scope(&a, Access::Read)]).is_err());
        drop(held);
        assert!(registry.try_acquire(vec![scope(&a, Access::Write)]).is_ok());
        assert!(registry.state.lock().unwrap().active.is_empty());
    }
}
