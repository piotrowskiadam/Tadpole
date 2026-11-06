use super::*;
use crate::visualization::snapshot::Indexability;

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
impl Bounds {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkKind {
    Area,
    Node,
    Cell,
    Dot,
}
pub struct Mark {
    pub bounds: Bounds,
    pub title: String,
    pub detail: String,
    pub tooltip: String,
    pub target: Target,
    pub kind: MarkKind,
    pub focal: bool,
    pub shade: f64,
    pub error: bool,
}
pub struct Edge {
    pub from: (f64, f64),
    pub to: (f64, f64),
    pub horizontal: bool,
    pub lane: f64,
}
pub struct Label {
    pub x: f64,
    pub y: f64,
    pub text: String,
}
#[derive(Debug, Clone, Copy)]
pub struct EvidenceRow {
    pub page: NodeId,
    pub target: Option<NodeId>,
}
pub struct PreparedView {
    pub view: View,
    pub width: f64,
    pub height: f64,
    pub title: String,
    pub subtitle: String,
    pub legend: String,
    pub status: String,
    pub marks: Vec<Mark>,
    pub edges: Vec<Edge>,
    pub labels: Vec<Label>,
    pub evidence: Arc<[EvidenceRow]>,
    pub ranks: Option<Arc<[f32]>>,
    pub depths: Arc<Depths>,
    pub focus: Option<NodeId>,
    hit_rows: Vec<Vec<usize>>,
}
impl PreparedView {
    pub fn hit_test(&self, x: f64, y: f64) -> Option<&Mark> {
        self.hit_rows
            .get((y.max(0.) / 16.) as usize)?
            .iter()
            .rev()
            .map(|&i| &self.marks[i])
            .find(|m| m.bounds.contains(x, y))
    }
    pub fn estimated_bytes(&self) -> usize {
        self.depths.depth.len() * 48
            + self.ranks.as_ref().map_or(0, |r| r.len() * 4)
            + self.evidence.len() * std::mem::size_of::<EvidenceRow>()
            + self
                .marks
                .iter()
                .map(|m| {
                    std::mem::size_of::<Mark>() + m.title.len() + m.detail.len() + m.tooltip.len()
                })
                .sum::<usize>()
            + self.edges.len() * std::mem::size_of::<Edge>()
            + self.labels.iter().map(|l| l.text.len() + 64).sum::<usize>()
            + self
                .hit_rows
                .iter()
                .map(|r| r.len() * 8 + 24)
                .sum::<usize>()
    }
    fn mark(
        &mut self,
        bounds: Bounds,
        title: String,
        detail: String,
        tooltip: String,
        target: Target,
        kind: MarkKind,
    ) {
        self.marks.push(Mark {
            bounds,
            title,
            detail,
            tooltip,
            target,
            kind,
            focal: false,
            shade: 0.07,
            error: false,
        });
    }
    fn label(&mut self, x: f64, y: f64, text: impl Into<String>) {
        self.labels.push(Label {
            x,
            y,
            text: text.into(),
        });
    }
}

pub(super) fn prepare(
    index: &Arc<ExplorerIndex>,
    r: &ViewRequest,
    ranks: Option<Arc<[f32]>>,
    depths: &Arc<Depths>,
    cancel: &impl Fn() -> bool,
) -> Option<PreparedView> {
    let n = index.snapshot.nodes.len();
    let query = r.query.trim().to_lowercase();
    let known_start = depths.depth.iter().any(Option::is_some);
    let mut matches = Vec::with_capacity(n);
    for node in index.snapshot.nodes.iter() {
        if node.id % 256 == 0 && cancel() {
            return None;
        }
        matches.push(
            (query.is_empty() || index.url(node.id).to_lowercase().contains(&query))
                && match r.filter {
                    PageFilter::All => true,
                    PageFilter::Indexable => node.indexability == Indexability::Indexable,
                    PageFilter::Issues => node.issue_mask != 0,
                    PageFilter::Errors => node.status_code >= 400,
                    PageFilter::Unreachable => {
                        known_start && depths.depth[node.id as usize].is_none()
                    }
                },
        );
    }
    let mut f = PreparedView {
        view: r.view,
        width: r.width.clamp(640, 8192) as f64,
        height: r.height.clamp(320, 8192) as f64,
        title: String::new(),
        subtitle: String::new(),
        legend: String::new(),
        status: String::new(),
        marks: vec![],
        edges: vec![],
        labels: vec![],
        evidence: Arc::from([]),
        ranks,
        depths: depths.clone(),
        focus: None,
        hit_rows: vec![],
    };
    let plot = Bounds {
        x: 32.,
        y: 100.,
        w: f.width - 64.,
        h: f.height - 158.,
    };
    let mut rows: Vec<_> = (0..n)
        .filter(|&i| matches[i])
        .map(|i| EvidenceRow {
            page: i as NodeId,
            target: None,
        })
        .collect();
    match r.view {
        View::Directory | View::Icicle => {
            directory(index, r, &matches, &mut f, plot, &mut rows, cancel)?
        }
        View::Paths => paths(index, r, &matches, &mut f, plot, &mut rows, cancel)?,
        View::Links => links(index, r, &matches, &mut f, plot, &mut rows),
        View::Matrix => matrix(index, r, &matches, &mut f, plot, &mut rows, cancel)?,
        View::Rank => rank(index, r, &matches, &mut f, plot, &mut rows),
    }
    if n == 0 {
        f.status = "No crawled URLs. Run a crawl or open a saved project, then Refresh.".into();
    } else if r.filter == PageFilter::Unreachable && !known_start {
        f.status = "Choose a crawled start URL before identifying unreachable pages.".into();
    } else if rows.is_empty() && f.status.is_empty() {
        f.status = "No matching evidence. Clear search or change the filter.".into();
    }
    f.evidence = rows.into();
    let matched = matches.iter().filter(|&&m| m).count();
    f.subtitle = format!(
        "{} · {} matching / {} crawled URLs · crawl data only",
        f.subtitle, matched, n
    );
    f.hit_rows = vec![vec![]; (f.height / 16.).ceil() as usize + 1];
    for (i, m) in f.marks.iter().enumerate() {
        let a = (m.bounds.y.max(0.) / 16.) as usize;
        let b = ((m.bounds.y + m.bounds.h).max(0.) / 16.) as usize;
        for y in a..=b.min(f.hit_rows.len() - 1) {
            f.hit_rows[y].push(i);
        }
    }
    Some(f)
}

fn page_mark(index: &ExplorerIndex, f: &mut PreparedView, id: NodeId, b: Bounds) {
    let node = &index.snapshot.nodes[id as usize];
    let url = index.url(id);
    let name = url::Url::parse(url)
        .ok()
        .map(|u| u.path().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| url.into());
    let depth = f.depths.depth[id as usize].map_or("unreachable / unknown start".into(), |d| {
        format!("{d} links from start")
    });
    f.mark(
        b,
        name,
        format!("HTTP {} · {}", node.status_code, depth),
        url.into(),
        Target::Page(id),
        MarkKind::Node,
    );
    f.marks.last_mut().unwrap().error = node.status_code >= 400;
}
struct Item {
    target: Target,
    name: String,
    detail: String,
    value: f64,
}
fn directory(
    index: &ExplorerIndex,
    r: &ViewRequest,
    mask: &[bool],
    f: &mut PreparedView,
    p: Bounds,
    rows: &mut Vec<EvidenceRow>,
    cancel: &impl Fn() -> bool,
) -> Option<()> {
    let root = r.directory.min(index.directories.len() - 1);
    rows.retain(|row| index.in_directory(row.page, root));
    let mut values = vec![0.; index.directories.len()];
    let mut counts = vec![0usize; values.len()];
    for row in rows.iter() {
        if row.page % 256 == 0 && cancel() {
            return None;
        }
        let node = &index.snapshot.nodes[row.page as usize];
        let value = match r.metric {
            IcicleWeightMetric::UrlCount => 1.,
            IcicleWeightMetric::PageSizeBytes => node.page_size_bytes as f64,
            IcicleWeightMetric::WordCount => node.word_count as f64,
            IcicleWeightMetric::IssueCount => f64::from(node.issue_mask != 0),
            IcicleWeightMetric::InternalRankSum => {
                f.ranks.as_ref().map_or(0., |v| v[row.page as usize] as f64)
            }
            IcicleWeightMetric::ResponseTimeMs => node.response_time_ms as f64,
        };
        let d = index.page_directory[row.page as usize];
        values[d] += value;
        counts[d] += 1;
    }
    let direct = values.clone();
    for d in (1..values.len()).rev() {
        let parent = index.directories[d].parent.unwrap();
        values[parent] += values[d];
        counts[parent] += counts[d];
    }
    f.title = if r.view == View::Directory {
        "Directory area"
    } else {
        "Directory icicle"
    }
    .into();
    f.subtitle = index.directories[root].path.clone();
    f.legend="Area/width = selected additive weight. URL containment, not hyperlinks. Click a group to focus; Up returns. Path groups stop at 64 components.".into();
    if values[root] <= 0. {
        f.status = "No positive values for this weight. Choose URL count or another metric.".into();
        return Some(());
    }
    let items_for = |d: usize| {
        let mut items: Vec<_> = index.directories[d]
            .children
            .iter()
            .filter(|&&c| values[c] > 0.)
            .map(|&c| Item {
                target: Target::Directory(c),
                name: index.directories[c].name.clone(),
                detail: format!("{} URLs · weight {:.3}", counts[c], values[c]),
                value: values[c],
            })
            .collect();
        if direct[d] > 0. {
            items.push(Item {
                target: Target::Bin(d),
                name: "Pages at this path".into(),
                detail: format!(
                    "{} direct URLs",
                    index.directories[d]
                        .direct
                        .iter()
                        .filter(|&&id| mask[id as usize])
                        .count()
                ),
                value: direct[d],
            });
        }
        items.sort_by(|a, b| b.value.total_cmp(&a.value).then(a.name.cmp(&b.name)));
        items
    };
    if r.view == View::Directory {
        let mut items = items_for(root);
        let mut omitted = std::collections::HashSet::new();
        if items.len() > 200 {
            let rest = items.split_off(199);
            omitted.extend(rest.iter().map(|i| i.target));
            items.push(Item {
                target: Target::Bin(usize::MAX),
                name: format!("Other · {} groups", rest.len()),
                detail: "All remaining URLs are in evidence".into(),
                value: rest.iter().map(|i| i.value).sum(),
            });
        }
        let selected_bin = if let Some(Target::Bin(d)) = r.scope {
            Some(d)
        } else {
            None
        };
        if let Some(d) = selected_bin {
            if d == usize::MAX {
                rows.retain(|row| {
                    let mut current = index.page_directory[row.page as usize];
                    if current == root {
                        return omitted.contains(&Target::Bin(root));
                    }
                    while let Some(parent) = index.directories[current].parent {
                        if parent == root {
                            return omitted.contains(&Target::Directory(current));
                        }
                        current = parent;
                    }
                    false
                });
            } else {
                rows.retain(|row| index.page_directory[row.page as usize] == d);
            }
        }
        for (item, b) in treemap(items, p) {
            f.mark(
                b,
                item.name,
                item.detail,
                index.directories[root].path.clone(),
                item.target,
                MarkKind::Area,
            );
        }
    } else {
        let row_height = (p.h / 6.).max(24.);
        let mut queue = VecDeque::from([(root, p.x, p.w, 0usize)]);
        while let Some((d, x, w, level)) = queue.pop_front() {
            if cancel() {
                return None;
            }
            if f.marks.len() >= 2000 || level >= 6 || w < 2. {
                continue;
            }
            f.mark(
                Bounds {
                    x,
                    y: p.y + level as f64 * row_height,
                    w,
                    h: row_height - 3.,
                },
                index.directories[d].name.clone(),
                format!("{} URLs", counts[d]),
                index.directories[d].path.clone(),
                Target::Directory(d),
                MarkKind::Area,
            );
            let mut cursor = x;
            for item in items_for(d) {
                let width = w * item.value / values[d];
                if let Target::Directory(c) = item.target {
                    queue.push_back((c, cursor, width, level + 1));
                } else if level < 5 && width >= 2. && f.marks.len() < 2000 {
                    f.mark(
                        Bounds {
                            x: cursor,
                            y: p.y + (level + 1) as f64 * row_height,
                            w: width,
                            h: row_height - 3.,
                        },
                        item.name,
                        item.detail,
                        "Direct pages; included in parent weight".into(),
                        item.target,
                        MarkKind::Area,
                    );
                }
                cursor += width;
            }
        }
        if let Some(Target::Bin(d)) = r.scope {
            rows.retain(|row| index.page_directory[row.page as usize] == d);
        }
        f.legend = format!(
            "{} blocks shown, at most six levels. {}",
            f.marks.len(),
            f.legend
        );
    }
    Some(())
}
fn treemap(mut items: Vec<Item>, mut b: Bounds) -> Vec<(Item, Bounds)> {
    let total: f64 = items.iter().map(|i| i.value).sum();
    if total <= 0. {
        return vec![];
    }
    for i in &mut items {
        i.value = i.value / total * b.w * b.h;
    }
    let mut pending: VecDeque<_> = items.into();
    let mut output = vec![];
    let worst = |row: &[Item], side: f64| {
        let sum: f64 = row.iter().map(|i| i.value).sum();
        let low = row.iter().map(|i| i.value).fold(f64::INFINITY, f64::min);
        let high = row.iter().map(|i| i.value).fold(0., f64::max);
        (side * side * high / (sum * sum)).max(sum * sum / (side * side * low))
    };
    while let Some(first) = pending.pop_front() {
        let mut row = vec![first];
        let side = b.w.min(b.h).max(0.001);
        while let Some(next) = pending.front() {
            let old = worst(&row, side);
            row.push(Item {
                target: next.target,
                name: String::new(),
                detail: String::new(),
                value: next.value,
            });
            let better = worst(&row, side) <= old;
            row.pop();
            if !better {
                break;
            }
            row.push(pending.pop_front().unwrap());
        }
        let total: f64 = row.iter().map(|i| i.value).sum();
        if b.w >= b.h {
            let w = (total / b.h).min(b.w);
            let mut y = b.y;
            for item in row {
                let h = item.value / w;
                output.push((item, Bounds { x: b.x, y, w, h }));
                y += h;
            }
            b.x += w;
            b.w = (b.w - w).max(0.001);
        } else {
            let h = (total / b.w).min(b.h);
            let mut x = b.x;
            for item in row {
                let w = item.value / h;
                output.push((item, Bounds { x, y: b.y, w, h }));
                x += w;
            }
            b.y += h;
            b.h = (b.h - h).max(0.001);
        }
    }
    output
}

fn paths(
    index: &ExplorerIndex,
    r: &ViewRequest,
    mask: &[bool],
    f: &mut PreparedView,
    p: Bounds,
    rows: &mut Vec<EvidenceRow>,
    cancel: &impl Fn() -> bool,
) -> Option<()> {
    f.title = "Link-depth tree".into();
    f.subtitle = "One shortest-path parent per reachable URL".into();
    f.legend="Edges = reconstructed shortest-path parents, not discovery order or all links. Double-click a page to focus its branch; Up returns.".into();
    let Some(root) = r
        .tree_focus
        .or_else(|| {
            index.node(if r.start_url.is_empty() {
                index.inferred_start().unwrap_or("")
            } else {
                &r.start_url
            })
        })
        .filter(|&id| (id as usize) < mask.len() && f.depths.depth[id as usize].is_some())
    else {
        f.status = "Set a crawled start URL to reconstruct shortest paths.".into();
        return Some(());
    };
    let mut subtree = vec![false; mask.len()];
    let mut q = VecDeque::from([root]);
    let mut order = vec![];
    while let Some(id) = q.pop_front() {
        if cancel() {
            return None;
        }
        subtree[id as usize] = true;
        order.push(id);
        q.extend(&f.depths.children[id as usize]);
    }
    rows.retain(|row| subtree[row.page as usize]);
    let mut relevant = mask.to_vec();
    for &id in order.iter().rev() {
        if relevant[id as usize]
            && let Some(parent) = f.depths.parent[id as usize]
        {
            relevant[parent as usize] = true;
        }
    }
    let per_level = ((p.w / 156.).floor() as usize).clamp(1, 20);
    let h = 44.;
    let step = (p.h / 4.).max(h + 32.);
    let mut level = vec![root];
    let mut previous = HashMap::<NodeId, Bounds>::new();
    let mut shown = 0;
    for depth in 0..4 {
        let mut next = vec![];
        let mut positions = HashMap::new();
        let count = level.len().min(per_level);
        let width = (p.w / count.max(1) as f64 - 16.).clamp(80., 176.);
        for (i, &id) in level.iter().take(per_level).enumerate() {
            let b = Bounds {
                x: p.x + (i as f64 + 0.5) * p.w / count.max(1) as f64 - width / 2.,
                y: p.y + depth as f64 * step,
                w: width,
                h,
            };
            if let Some(parent) = f.depths.parent[id as usize].and_then(|id| previous.get(&id)) {
                f.edges.push(Edge {
                    from: (
                        parent.x + parent.w * (i + 1) as f64 / (count + 1) as f64,
                        parent.y + parent.h,
                    ),
                    to: (b.x + b.w / 2., b.y),
                    horizontal: false,
                    lane: parent.y + parent.h + 10. + (i % 5) as f64 * 4.,
                });
            }
            page_mark(index, f, id, b);
            let children = &f.depths.children[id as usize];
            f.marks.last_mut().unwrap().detail = format!(
                "{} children · {} links",
                children.len(),
                f.depths.depth[id as usize].unwrap_or(0)
            );
            next.extend(children.iter().copied().filter(|&c| relevant[c as usize]));
            positions.insert(id, b);
            if mask[id as usize] {
                shown += 1;
            }
        }
        previous = positions;
        level = next;
        if level.is_empty() {
            break;
        }
    }
    f.focus = Some(root);
    f.legend = format!(
        "{} of {} matching branch URLs drawn; expand by focusing a page. {}",
        shown,
        rows.len(),
        f.legend
    );
    Some(())
}

fn links(
    index: &ExplorerIndex,
    r: &ViewRequest,
    mask: &[bool],
    f: &mut PreparedView,
    p: Bounds,
    rows: &mut Vec<EvidenceRow>,
) {
    f.title = "URL link neighborhood".into();
    let focus = index
        .node(&r.selected_url)
        .or_else(|| index.node(&r.start_url))
        .or_else(|| rows.first().map(|r| r.page));
    let Some(id) = focus else {
        f.status = "Choose a crawled URL from the evidence list.".into();
        return;
    };
    f.focus = Some(id);
    f.subtitle = index.url(id).into();
    let incoming = index.snapshot.get_incoming_neighbors(id);
    let outgoing = index.snapshot.get_outgoing_neighbors(id);
    let inc: Vec<_> = incoming
        .iter()
        .copied()
        .filter(|&p| mask[p as usize] && p != id)
        .collect();
    let out: Vec<_> = outgoing
        .iter()
        .copied()
        .filter(|&p| mask[p as usize] && p != id)
        .collect();
    rows.clear();
    rows.extend(inc.iter().map(|&p| EvidenceRow {
        page: p,
        target: Some(id),
    }));
    rows.extend(out.iter().map(|&p| EvidenceRow {
        page: id,
        target: Some(p),
    }));
    if incoming.contains(&id) {
        rows.push(EvidenceRow {
            page: id,
            target: Some(id),
        });
    }
    let cap = ((p.h / 64.).floor() as usize).clamp(1, 124);
    let w = (p.w * 0.25).min(240.);
    let center = Bounds {
        x: p.x + (p.w - w) / 2.,
        y: p.y + p.h / 2. - 28.,
        w,
        h: 56.,
    };
    page_mark(index, f, id, center);
    f.marks.last_mut().unwrap().focal = true;
    for (side, ids) in [(&inc, false), (&out, true)]
        .into_iter()
        .map(|(a, b)| (b, a))
    {
        let count = ids.len().min(cap);
        for (i, &neighbor) in ids.iter().take(cap).enumerate() {
            let b = Bounds {
                x: if side { p.x + p.w - w } else { p.x },
                y: p.y + (i as f64 + 0.5) * p.h / count.max(1) as f64 - 24.,
                w,
                h: 48.,
            };
            page_mark(index, f, neighbor, b);
            let port = center.y + 8. + (i + 1) as f64 * 40. / (count + 1) as f64;
            f.edges.push(Edge {
                from: if side {
                    (center.x + w, port)
                } else {
                    (b.x + w, b.y + 24.)
                },
                to: if side {
                    (b.x, b.y + 24.)
                } else {
                    (center.x, port)
                },
                horizontal: true,
                lane: if side {
                    center.x
                        + w
                        + (p.x + p.w - w - center.x - w)
                            * (0.25 + 0.5 * (i + 1) as f64 / (count + 1) as f64)
                } else {
                    p.x + w
                        + (center.x - p.x - w) * (0.25 + 0.5 * (i + 1) as f64 / (count + 1) as f64)
                },
            });
        }
    }
    f.legend = format!(
        "Arrows = unique URL pairs. Incoming {}/{} matching ({} known); outgoing {}/{} matching ({} known). Self-links appear in evidence. Dashed node = HTTP error.",
        inc.len().min(cap),
        inc.len(),
        incoming.len(),
        out.len().min(cap),
        out.len(),
        outgoing.len()
    );
}

fn matrix(
    index: &ExplorerIndex,
    r: &ViewRequest,
    mask: &[bool],
    f: &mut PreparedView,
    p: Bounds,
    rows: &mut Vec<EvidenceRow>,
    cancel: &impl Fn() -> bool,
) -> Option<()> {
    f.title = "Section link matrix".into();
    f.subtitle = "Rows: sources · Columns: targets · unique URL pairs".into();
    let mut counts = vec![0usize; index.sections.len()];
    for row in rows.iter() {
        counts[index.page_section[row.page as usize]] += 1;
    }
    let mut ranked: Vec<_> = (0..counts.len()).filter(|&i| counts[i] > 0).collect();
    ranked.sort_by(|&a, &b| {
        counts[b]
            .cmp(&counts[a])
            .then(index.sections[a].cmp(&index.sections[b]))
    });
    let cap = ((p.h / 28.).min((p.w - 200.) / 44.) as usize).clamp(1, 40);
    let has_other = ranked.len() > cap;
    let keep = if has_other {
        cap.saturating_sub(1)
    } else {
        ranked.len()
    };
    let mut groups = ranked[..keep].to_vec();
    if has_other {
        groups.push(usize::MAX);
    }
    let m = groups.len();
    if m == 0 {
        return Some(());
    }
    let mut mapping = vec![usize::MAX; counts.len()];
    for (i, &s) in groups.iter().enumerate() {
        if s != usize::MAX {
            mapping[s] = i;
        }
    }
    if has_other {
        for &s in &ranked[keep..] {
            mapping[s] = m - 1;
        }
    }
    let mut cells = vec![0usize; m * m];
    let scope = if let Some(Target::Cell(a, b)) = r.scope {
        Some((a, b))
    } else {
        None
    };
    if scope.is_some() {
        rows.clear();
    }
    for (i, edge) in index.snapshot.edges.iter().enumerate() {
        if i % 2048 == 0 && cancel() {
            return None;
        }
        let a = edge.source as usize;
        let b = edge.target as usize;
        if !mask[a] || !mask[b] {
            continue;
        }
        let sa = mapping[index.page_section[a]];
        let sb = mapping[index.page_section[b]];
        if sa >= m || sb >= m {
            continue;
        }
        cells[sa * m + sb] += 1;
        if scope == Some((groups[sa], groups[sb])) {
            rows.push(EvidenceRow {
                page: edge.source,
                target: Some(edge.target),
            });
        }
    }
    let w = (p.w - 200.) / m as f64;
    let h = p.h / m as f64;
    let mut denominators = vec![0usize; m];
    for &section in &ranked {
        denominators[mapping[section]] += counts[section];
    }
    let value = |a: usize, b: usize| {
        if r.normalize {
            cells[a * m + b] as f64 / denominators[a].max(1) as f64
        } else {
            cells[a * m + b] as f64
        }
    };
    let max = (0..m)
        .flat_map(|a| (0..m).map(move |b| (a, b)))
        .map(|(a, b)| value(a, b))
        .fold(0., f64::max)
        .max(0.000001);
    if r.normalize {
        f.subtitle =
            "Rows: sources · Columns: targets · unique pairs per crawled source page".into();
    }
    for (a, &group) in groups.iter().enumerate() {
        let name = if group == usize::MAX {
            format!("Other ({} sections)", ranked.len() - keep)
        } else {
            index.sections[group].clone()
        };
        f.label(
            p.x,
            p.y + (a as f64 + 0.6) * h,
            format!("{} {}", a + 1, name),
        );
        f.label(
            p.x + 200. + (a as f64 + 0.5) * w,
            p.y - 12.,
            (a + 1).to_string(),
        );
        for b in 0..m {
            let count = cells[a * m + b];
            f.mark(
                Bounds {
                    x: p.x + 200. + b as f64 * w,
                    y: p.y + a as f64 * h,
                    w,
                    h,
                },
                if r.normalize {
                    format!("{:.2}", value(a, b))
                } else {
                    count.to_string()
                },
                String::new(),
                format!(
                    "{name} → {}: {count} unique URL pairs; {} source pages",
                    if groups[b] == usize::MAX {
                        "Other"
                    } else {
                        &index.sections[groups[b]]
                    },
                    denominators[a]
                ),
                Target::Cell(group, groups[b]),
                MarkKind::Cell,
            );
            f.marks.last_mut().unwrap().shade = 0.04 + 0.12 * value(a, b) / max;
            f.marks.last_mut().unwrap().focal = scope == Some((group, groups[b]));
        }
    }
    f.legend = format!(
        "{} section groups, including Other when needed; every matching pair is counted. Source and target must match filters. Diagonal = within-section links.",
        m
    );
    Some(())
}

fn rank(
    index: &ExplorerIndex,
    r: &ViewRequest,
    _mask: &[bool],
    f: &mut PreparedView,
    p: Bounds,
    rows: &mut Vec<EvidenceRow>,
) {
    f.title = "Depth and internal rank".into();
    f.subtitle = "Shortest-path depth vs whole-crawl internal rank share (%)".into();
    let Some(ranks) = f.ranks.clone() else {
        return;
    };
    if !f.depths.depth.iter().any(Option::is_some) {
        f.status = "Set a crawled start URL to plot shortest-path depth.".into();
        return;
    }
    let missing = rows
        .iter()
        .filter(|row| f.depths.depth[row.page as usize].is_none())
        .count();
    rows.retain(|row| f.depths.depth[row.page as usize].is_some());
    let max_depth = rows
        .iter()
        .filter_map(|row| f.depths.depth[row.page as usize])
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let plotted_count = rows.len();
    let max_rank = rows
        .iter()
        .map(|row| ranks[row.page as usize] as f64)
        .fold(0., f64::max)
        .max(0.000001);
    let plot = Bounds {
        x: p.x + 48.,
        y: p.y + 12.,
        w: p.w - 72.,
        h: p.h - 40.,
    };
    for t in 0..=4 {
        let v = t as f64 / 4.;
        f.label(
            plot.x + v * plot.w,
            plot.y + plot.h + 24.,
            format!("{:.0}", v * max_depth),
        );
        f.label(
            p.x,
            plot.y + (1. - v) * plot.h,
            format!("{:.2}%", v * max_rank * 100.),
        );
    }
    let mut bins: Vec<Vec<NodeId>> = vec![vec![]; 80 * 40];
    for row in rows.iter() {
        let x = f.depths.depth[row.page as usize].unwrap() as f64 / max_depth;
        let y = ranks[row.page as usize] as f64 / max_rank;
        if rows.len() <= 1000 {
            f.mark(
                Bounds {
                    x: plot.x + x * plot.w - 5.,
                    y: plot.y + (1. - y) * plot.h - 5.,
                    w: 10.,
                    h: 10.,
                },
                String::new(),
                String::new(),
                format!(
                    "{} · {} links · {:.5}% rank",
                    index.url(row.page),
                    f.depths.depth[row.page as usize].unwrap(),
                    ranks[row.page as usize] * 100.
                ),
                Target::Page(row.page),
                MarkKind::Dot,
            );
        } else {
            let bx = ((x * 80.) as usize).min(79);
            let by = (((1. - y) * 40.) as usize).min(39);
            bins[by * 80 + bx].push(row.page);
        }
    }
    if rows.len() > 1000 {
        let max = bins.iter().map(Vec::len).max().unwrap_or(1).max(1) as f64;
        for (i, bin) in bins.iter().enumerate().filter(|(_, b)| !b.is_empty()) {
            f.mark(
                Bounds {
                    x: plot.x + (i % 80) as f64 * plot.w / 80.,
                    y: plot.y + (i / 80) as f64 * plot.h / 40.,
                    w: plot.w / 80.,
                    h: plot.h / 40.,
                },
                String::new(),
                String::new(),
                format!(
                    "{} URLs in this depth/rank bin; click for evidence",
                    bin.len()
                ),
                Target::Bin(i),
                MarkKind::Cell,
            );
            f.marks.last_mut().unwrap().shade = 0.06 + 0.10 * (bin.len() as f64 / max);
        }
        if let Some(Target::Bin(i)) = r.scope
            && let Some(bin) = bins.get(i)
        {
            *rows = bin
                .iter()
                .map(|&page| EvidenceRow { page, target: None })
                .collect();
        }
    }
    f.legend = format!(
        "{} reachable matching URLs; {} matching URLs without a path excluded from the plot. {}. Internal rank is a local graph score, not Google's PageRank.",
        plotted_count,
        missing,
        if bins.iter().any(|b| !b.is_empty()) {
            "80 × 40 density bins; exact counts, no sampling"
        } else {
            "One dot per URL"
        }
    );
}
