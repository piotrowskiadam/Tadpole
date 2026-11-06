//! Native view preparation. One worker, one newest pending request, immutable
//! frames. GTK never builds a graph or performs graph traversal while painting.
mod frame;
use super::pagerank::{PageRankConfig, calculate_internal_rank_cancellable};
use super::{GraphSnapshot, IcicleWeightMetric, NodeId, SnapshotBuilder};
use crate::state::CrawlResult;
pub use frame::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum View {
    #[default]
    Directory,
    Icicle,
    Paths,
    Links,
    Matrix,
    Rank,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PageFilter {
    #[default]
    All,
    Indexable,
    Issues,
    Errors,
    Unreachable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Page(NodeId),
    Directory(usize),
    Cell(usize, usize),
    Bin(usize),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewRequest {
    pub view: View,
    pub metric: IcicleWeightMetric,
    pub filter: PageFilter,
    pub normalize: bool,
    pub query: String,
    pub start_url: String,
    pub selected_url: String,
    pub directory: usize,
    pub tree_focus: Option<NodeId>,
    pub scope: Option<Target>,
    pub width: u32,
    pub height: u32,
}
impl Default for ViewRequest {
    fn default() -> Self {
        Self {
            view: View::Directory,
            metric: IcicleWeightMetric::UrlCount,
            filter: PageFilter::All,
            normalize: false,
            query: String::new(),
            start_url: String::new(),
            selected_url: String::new(),
            directory: 0,
            tree_focus: None,
            scope: None,
            width: 1000,
            height: 440,
        }
    }
}

#[derive(Debug)]
pub struct Directory {
    pub name: String,
    pub path: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub direct: Vec<NodeId>,
    pub depth: usize,
}
#[derive(Debug)]
pub struct ExplorerIndex {
    pub snapshot: GraphSnapshot,
    pub directories: Vec<Directory>,
    pub page_directory: Vec<usize>,
    pub sections: Vec<String>,
    pub page_section: Vec<usize>,
    urls: HashMap<String, NodeId>,
}
impl ExplorerIndex {
    pub fn node(&self, url: &str) -> Option<NodeId> {
        self.urls.get(url).copied().or_else(|| {
            let mut parsed = url::Url::parse(&crate::crawler::clean_seed_url(url)).ok()?;
            parsed.set_fragment(None);
            self.urls.get(parsed.as_str()).copied()
        })
    }
    pub fn url(&self, id: NodeId) -> &str {
        self.snapshot.node_url(&self.snapshot.nodes[id as usize])
    }
    pub fn inferred_start(&self) -> Option<&str> {
        let mut roots = self.snapshot.nodes.iter().filter(|n| n.crawl_depth == 0);
        let first = roots.next()?;
        if roots.next().is_some() {
            None
        } else {
            Some(self.snapshot.node_url(first))
        }
    }
    pub fn in_directory(&self, page: NodeId, directory: usize) -> bool {
        let mut current = Some(self.page_directory[page as usize]);
        while let Some(i) = current {
            if i == directory {
                return true;
            }
            current = self.directories[i].parent;
        }
        false
    }
    fn build(snapshot: GraphSnapshot, cancelled: &impl Fn() -> bool) -> Option<Self> {
        let mut result = Self {
            snapshot,
            directories: vec![Directory {
                name: "All origins".into(),
                path: "All origins".into(),
                parent: None,
                children: vec![],
                direct: vec![],
                depth: 0,
            }],
            page_directory: vec![],
            sections: vec![],
            page_section: vec![],
            urls: HashMap::new(),
        };
        let mut paths = HashMap::new();
        let mut sections = HashMap::new();
        for node in result.snapshot.nodes.iter() {
            if node.id % 256 == 0 && cancelled() {
                return None;
            }
            let raw = result.snapshot.node_url(node);
            result.urls.insert(raw.into(), node.id);
            let parsed = url::Url::parse(raw).ok();
            let origin = parsed
                .as_ref()
                .map(|u| u.origin().ascii_serialization())
                .unwrap_or_else(|| "Unparsed URLs".into());
            let path = parsed.as_ref().map_or("", |u| u.path());
            let segments: Vec<_> = path.split('/').filter(|p| !p.is_empty()).collect();
            let mut current = 0;
            for name in std::iter::once(origin.as_str()).chain(segments.iter().copied().take(64)) {
                let key = (current, name.to_string());
                current = *paths.entry(key).or_insert_with(|| {
                    let id = result.directories.len();
                    let path = if current == 0 {
                        name.into()
                    } else {
                        format!("{}/{}", result.directories[current].path, name)
                    };
                    result.directories.push(Directory {
                        name: name.into(),
                        path,
                        parent: Some(current),
                        children: vec![],
                        direct: vec![],
                        depth: result.directories[current].depth + 1,
                    });
                    result.directories[current].children.push(id);
                    id
                });
            }
            result.directories[current].direct.push(node.id);
            result.page_directory.push(current);
            let section_name = format!(
                "{}/{}",
                origin,
                segments.first().copied().unwrap_or("(root)")
            );
            let next = result.sections.len();
            let section = *sections.entry(section_name.clone()).or_insert_with(|| {
                result.sections.push(section_name);
                next
            });
            result.page_section.push(section);
        }
        Some(result)
    }
}

pub struct ExplorerEngine {
    pub index: Arc<ExplorerIndex>,
    ranks: Option<Arc<[f32]>>,
    depths: Option<(String, Arc<Depths>)>,
    cache: VecDeque<(ViewRequest, Arc<PreparedView>)>,
}
pub struct Depths {
    pub depth: Vec<Option<u32>>,
    pub parent: Vec<Option<NodeId>>,
    pub children: Vec<Vec<NodeId>>,
}
impl ExplorerEngine {
    pub fn new(mut facts: Vec<CrawlResult>, cancelled: &impl Fn() -> bool) -> Option<Self> {
        if cancelled() {
            return None;
        }
        facts.sort_by(|a, b| a.url.cmp(&b.url));
        let snapshot = SnapshotBuilder::build_from_results_cancellable(&facts, cancelled)?;
        let index = Arc::new(ExplorerIndex::build(snapshot, cancelled)?);
        Some(Self {
            index,
            ranks: None,
            depths: None,
            cache: VecDeque::new(),
        })
    }
    pub fn prepare(
        &mut self,
        request: &ViewRequest,
        cancelled: &impl Fn() -> bool,
    ) -> Option<Arc<PreparedView>> {
        if cancelled() {
            return None;
        }
        if let Some(i) = self.cache.iter().position(|(r, _)| r == request) {
            let item = self.cache.remove(i).unwrap();
            let frame = item.1.clone();
            self.cache.push_back(item);
            return Some(frame);
        }
        if (request.view == View::Rank
            || (matches!(request.view, View::Directory | View::Icicle)
                && request.metric == IcicleWeightMetric::InternalRankSum))
            && self.ranks.is_none()
        {
            self.ranks = Some(
                calculate_internal_rank_cancellable(
                    &self.index.snapshot,
                    &PageRankConfig::default(),
                    cancelled,
                )?
                .into(),
            );
        }
        let start = if request.start_url.is_empty() {
            self.index.inferred_start().unwrap_or("")
        } else {
            &request.start_url
        };
        if self.depths.as_ref().is_none_or(|(s, _)| s != start) {
            let n = self.index.snapshot.nodes.len();
            let mut depths = Depths {
                depth: vec![None; n],
                parent: vec![None; n],
                children: vec![vec![]; n],
            };
            if let Some(root) = self.index.node(start) {
                let mut queue = VecDeque::from([root]);
                depths.depth[root as usize] = Some(0);
                while let Some(source) = queue.pop_front() {
                    if cancelled() {
                        return None;
                    }
                    for &target in self.index.snapshot.get_outgoing_neighbors(source) {
                        if depths.depth[target as usize].is_none() {
                            depths.depth[target as usize] =
                                Some(depths.depth[source as usize].unwrap() + 1);
                            depths.parent[target as usize] = Some(source);
                            queue.push_back(target);
                        } else if depths.depth[target as usize]
                            == Some(depths.depth[source as usize].unwrap() + 1)
                            && depths.parent[target as usize].is_some_and(|parent| source < parent)
                        {
                            depths.parent[target as usize] = Some(source);
                        }
                    }
                }
            }
            for (child, parent) in depths.parent.iter().enumerate() {
                if let Some(parent) = parent {
                    depths.children[*parent as usize].push(child as NodeId);
                }
            }
            self.depths = Some((start.into(), Arc::new(depths)));
        }
        let frame = Arc::new(frame::prepare(
            &self.index,
            request,
            self.ranks.clone(),
            &self.depths.as_ref().unwrap().1,
            cancelled,
        )?);
        if cancelled() {
            return None;
        }
        let bytes = frame.estimated_bytes();
        if bytes <= 32 * 1024 * 1024 {
            while !self.cache.is_empty()
                && (self.cache.len() >= 8
                    || self
                        .cache
                        .iter()
                        .map(|(_, f)| f.estimated_bytes())
                        .sum::<usize>()
                        + bytes
                        > 32 * 1024 * 1024)
            {
                self.cache.pop_front();
            }
            self.cache.push_back((request.clone(), frame.clone()));
        }
        Some(frame)
    }
}

pub struct ReadyView {
    pub revision: u64,
    pub request: ViewRequest,
    pub index: Arc<ExplorerIndex>,
    pub frame: Arc<PreparedView>,
}
pub enum SessionEvent {
    Ready(ReadyView),
    Failed { revision: u64, message: String },
}
struct Control {
    pending: Mutex<Option<(u64, ViewRequest)>>,
    result: Mutex<Option<SessionEvent>>,
    wake: Condvar,
    latest: AtomicU64,
    refresh: AtomicBool,
    closed: AtomicBool,
}
pub struct VisualizationSession {
    control: Arc<Control>,
}
impl VisualizationSession {
    pub fn new(source: impl Fn() -> Vec<CrawlResult> + Send + 'static) -> Self {
        let control = Arc::new(Control {
            pending: Mutex::new(None),
            result: Mutex::new(None),
            wake: Condvar::new(),
            latest: AtomicU64::new(0),
            refresh: AtomicBool::new(true),
            closed: AtomicBool::new(false),
        });
        let c = control.clone();
        std::thread::spawn(move || {
            let mut engine: Option<ExplorerEngine> = None;
            loop {
                let job = {
                    let mut pending = c.pending.lock().unwrap();
                    while pending.is_none() && !c.closed.load(Ordering::Acquire) {
                        pending = c.wake.wait(pending).unwrap();
                    }
                    if c.closed.load(Ordering::Acquire) {
                        break;
                    }
                    pending.take().unwrap()
                };
                let (revision, request) = job;
                let cancelled = || {
                    c.closed.load(Ordering::Acquire) || c.latest.load(Ordering::Acquire) != revision
                };
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if c.refresh.swap(false, Ordering::AcqRel) {
                        engine = None;
                    }
                    if engine.is_none() {
                        engine = ExplorerEngine::new(source(), &cancelled);
                    }
                    let e = engine.as_mut()?;
                    let frame = e.prepare(&request, &cancelled)?;
                    Some(ReadyView {
                        revision,
                        request: request.clone(),
                        index: e.index.clone(),
                        frame,
                    })
                }));
                if !cancelled() {
                    let event = match outcome {
                        Ok(Some(ready)) => Some(SessionEvent::Ready(ready)),
                        Ok(None) => None,
                        Err(_) => {
                            engine = None;
                            Some(SessionEvent::Failed {
                                revision,
                                message: "Visualization preparation failed. Refresh to retry."
                                    .into(),
                            })
                        }
                    };
                    if let Some(event) = event {
                        *c.result.lock().unwrap() = Some(event);
                    }
                }
            }
        });
        Self { control }
    }
    pub fn request(&self, request: ViewRequest, refresh: bool) -> u64 {
        let revision = self.control.latest.fetch_add(1, Ordering::AcqRel) + 1;
        if refresh {
            self.control.refresh.store(true, Ordering::Release);
        }
        *self.control.pending.lock().unwrap() = Some((revision, request));
        self.control.wake.notify_one();
        revision
    }
    pub fn poll(&self) -> Option<SessionEvent> {
        let event = self.control.result.lock().unwrap().take()?;
        let revision = match &event {
            SessionEvent::Ready(r) => r.revision,
            SessionEvent::Failed { revision, .. } => *revision,
        };
        (revision == self.control.latest.load(Ordering::Acquire)).then_some(event)
    }
    pub fn cancel(&self) {
        self.control.latest.fetch_add(1, Ordering::AcqRel);
        *self.control.pending.lock().unwrap() = None;
    }
    pub fn close(&self) {
        self.control.closed.store(true, Ordering::Release);
        self.cancel();
        self.control.wake.notify_one();
    }
}
impl Drop for VisualizationSession {
    fn drop(&mut self) {
        self.close();
    }
}
