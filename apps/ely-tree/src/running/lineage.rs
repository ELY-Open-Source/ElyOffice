//! Git's descendants: parent id and start time (R20.96).

/// A process as a snapshot lists it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Listed {
    pub pid: u32,
    pub parent: u32,
    /// Start time; None if unreadable.
    pub created: Option<u64>,
    /// A console host, which gets no Ctrl+C.
    pub host: bool,
}

/// `root`, born at `born`, and descendants no older; unread ones.
pub(super) fn descendants(root: u32, born: u64, all: &[Listed]) -> (Vec<(u32, bool)>, Vec<u32>) {
    let mut found = vec![(root, false, Some(born))];
    let mut unread = Vec::new();
    // Windows keeps dead parents' ids; ids are reused.
    loop {
        let before = found.len();
        for listed in all {
            if found.iter().any(|&(seen, ..)| seen == listed.pid) || unread.contains(&listed.pid) {
                continue;
            }
            let Some(&(_, _, parent)) = found.iter().find(|&&(seen, ..)| seen == listed.parent) else {
                continue;
            };
            match (listed.created, parent) {
                (Some(child), Some(parent)) if child >= parent => found.push((listed.pid, listed.host, Some(child))),
                // Older than its parent: a reused id.
                (Some(_), Some(_)) => {}
                _ => unread.push(listed.pid),
            }
        }
        if found.len() == before {
            return (found.into_iter().map(|(pid, host, _)| (pid, host)).collect(), unread);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Listed, descendants};

    fn listed(pid: u32, parent: u32, created: Option<u64>) -> Listed {
        Listed { pid, parent, created, host: false }
    }

    fn pids(found: &[(u32, bool)]) -> Vec<u32> {
        let mut pids: Vec<u32> = found.iter().map(|&(pid, _)| pid).collect();
        pids.sort_unstable();
        pids
    }

    /// The VM's explorer: its parent 6128 died long ago.
    fn incident() -> Vec<Listed> {
        vec![
            // Children before parents, as a snapshot may list them.
            listed(4100, 4000, Some(200)),
            listed(4200, 4100, Some(300)),
            listed(4000, 6128, Some(100)),
            listed(5100, 5000, Some(1_200)),
            listed(5000, 6128, Some(1_100)),
            Listed { pid: 5001, parent: 6128, created: Some(1_000), host: true },
            listed(6128, 900, Some(1_000)),
        ]
    }

    #[test]
    fn git_given_explorer_s_stale_parent_id_never_takes_explorer() {
        let (found, unread) = descendants(6128, 1_000, &incident());
        assert_eq!(pids(&found), [5000, 5001, 5100, 6128]);
        assert!(found.contains(&(5001, true)), "the console host is marked: {found:?}");
        assert!(unread.is_empty(), "{unread:?}");
    }

    #[test]
    fn a_child_started_in_its_parent_s_tick_counts() {
        let all = [listed(10, 1, Some(500)), listed(11, 10, Some(500))];
        assert_eq!(pids(&descendants(10, 500, &all).0), [10, 11]);
    }

    #[test]
    fn an_unreadable_process_and_its_children_are_left_out_and_named() {
        let mut all = incident();
        all.push(listed(5200, 6128, None));
        all.push(listed(5300, 5200, Some(1_300)));
        let (found, unread) = descendants(6128, 1_000, &all);
        assert_eq!(pids(&found), [5000, 5001, 5100, 6128]);
        assert_eq!(unread, [5200]);
    }

    #[test]
    fn git_gone_from_the_listing_still_judges_its_helpers() {
        // Git exited first; its time came through its handle.
        let all: Vec<Listed> = incident().into_iter().filter(|listed| listed.pid != 6128).collect();
        let (found, unread) = descendants(6128, 1_000, &all);
        assert_eq!(pids(&found), [5000, 5001, 5100, 6128]);
        assert!(unread.is_empty(), "{unread:?}");
    }
}
