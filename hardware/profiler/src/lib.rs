use std::{
    collections::HashMap,
    ops::Range,
    time::Duration,
};

pub use wgpu_profiler as gpu;
use wgpu_profiler::{
    GpuProfiler,
    GpuTimerQueryResult,
};

#[must_use = "Stream result must be checked for failure"]
pub enum StreamResult {
    Success,
    Empty,
    Disabled,
    Failure,
}

struct Delta {
    first: Duration,
}

impl Delta {
    pub fn new(timings: &[GpuTimerQueryResult]) -> Self {
        assert!(!timings.is_empty());

        let start_time = timings.first().expect("timings.len() > 0").time.start;
        let first = Duration::from_secs_f64(start_time);

        Self { first }
    }

    fn range_to_nanos(&self, range: &Range<f64>) -> (i64, i64) {
        let Range { start, end } = range;

        // convert to duration first
        let start = Duration::from_secs_f64(*start);
        let end = Duration::from_secs_f64(*end);

        // the ranges of time are measured from epoch
        let start = start - self.first;
        let end = end - self.first;

        let start = duration_ns(start);
        let end = duration_ns(end);

        (start, end)
    }
}

pub type IdCache = HashMap<String, puffin::ScopeId>;

pub trait PuffinStream {
    fn send_to_puffin(
        &mut self,
        start_time_ns: i64,
        ns_per_frame: f32,
        id_cache: Option<&mut IdCache>,
    ) -> StreamResult;
}

impl PuffinStream for GpuProfiler {
    #[profiling::function]
    fn send_to_puffin(
        &mut self,
        start_time_ns: i64,
        ns_per_frame: f32,
        id_cache: Option<&mut IdCache>,
    ) -> StreamResult {
        if !puffin::are_scopes_on() {
            return StreamResult::Disabled;
        }

        if let Some(timings) = self.process_finished_frame(ns_per_frame) {
            if timings.is_empty() {
                // no point adding scopes if there aren't any!
                return StreamResult::Empty;
            }

            // create a stream to write scopes to
            let mut stream = puffin::Stream::default();

            // give puffin details of the scopes we're about to allocate
            let ids = {
                // lock the profiler
                let mut profiler = puffin::GlobalProfiler::lock();

                let mut scope_names = Vec::new();

                // push all of the gpu timings
                for res in &timings {
                    add_scope_names(&mut scope_names, res)
                }

                if let Some(cache) = id_cache {
                    let mut ids = Vec::new();

                    // go through and try and get ids from the cache first
                    for name in scope_names {
                        let id = cache.entry(name.clone()).or_insert_with(|| {
                            // scope with this name doesn't exist, register it
                            let id = profiler.register_user_scopes(&[
                                puffin::ScopeDetails::from_scope_name(name),
                            ]);

                            assert_eq!(id.len(), 1);

                            // extract the id
                            let &[id] = id.as_slice() else {
                                unreachable!();
                            };

                            id
                        });

                        ids.push(*id);
                    }

                    ids
                } else {
                    // no cache, just register new scopes all at once
                    let scopes = scope_names
                        .into_iter()
                        .map(puffin::ScopeDetails::from_scope_name)
                        .collect::<Vec<_>>();

                    profiler.register_user_scopes(&scopes)
                }
            };

            // add each scope with their ids into the stream
            {
                let delta = Delta::new(&timings);

                // write the timings to the stream
                let mut index = 0;
                for res in &timings {
                    write_timings(&mut stream, start_time_ns, res, &ids, &delta, &mut index);
                }
            }

            {
                // lock the profiler
                let mut profiler = puffin::GlobalProfiler::lock();

                // finally, report the scopes to puffin using the stream
                profiler.report_user_scopes(
                    // the "gpu" is it's own thread
                    puffin::ThreadInfo {
                        start_time_ns: None,
                        name: "gpu".to_owned(),
                    },
                    &puffin::StreamInfo::parse(stream)
                        .unwrap()
                        .as_stream_into_ref(),
                );
            }

            StreamResult::Success
        } else {
            StreamResult::Failure
        }
    }
}

fn write_timings(
    stream: &mut puffin::Stream,
    offset: i64,
    result: &GpuTimerQueryResult,
    ids: &[puffin::ScopeId],
    timing: &Delta,
    index: &mut usize,
) {
    let (start, end) = timing.range_to_nanos(&result.time);

    let (parent_scope, child_offset) = stream.begin_scope(|| offset + start, ids[*index], "");

    for child in &result.nested_queries {
        *index += 1;
        write_timings(stream, child_offset, child, ids, timing, index);
    }

    stream.end_scope(parent_scope, offset + end);
}

fn add_scope_names(scopes: &mut Vec<String>, result: &GpuTimerQueryResult) {
    // the only details we can extract is the from label,
    // there are no function, line, file details available.
    scopes.push(result.label.clone());

    // process the children
    // have to do this in the same order as `write_timings`
    for child in &result.nested_queries {
        add_scope_names(scopes, child);
    }
}

fn duration_ns(d: Duration) -> i64 {
    d.as_nanos() as i64
}
