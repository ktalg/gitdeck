use crate::repo::CommitSummary;

pub const LANE_COLOR_SLOT_COUNT: u8 = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaneSpan {
    pub lane: u8,
    pub continues_above: bool,
    pub continues_below: bool,
    pub color_slot: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaneRow {
    pub commit_lane: u8,
    pub lanes: Vec<LaneSpan>,
}

pub fn assign_lanes(commits: &[CommitSummary]) -> Vec<LaneRow> {
    let mut lane_table = LaneTable::default();
    commits
        .iter()
        .map(|commit| lane_table.assign_row(commit))
        .collect()
}

struct Lane {
    expected_id: Option<String>,
    color_slot: u8,
}

#[derive(Default)]
struct LaneTable {
    lanes: Vec<Lane>,
    next_color_slot: u8,
}

impl LaneTable {
    fn assign_row(&mut self, commit: &CommitSummary) -> LaneRow {
        let occupied_above: Vec<bool> = self
            .lanes
            .iter()
            .map(|lane| lane.expected_id.is_some())
            .collect();
        let expecting = self.lanes_expecting(&commit.id);
        let commit_lane = match expecting.first() {
            Some(lane) => *lane,
            None => self.allocate_free_lane(),
        };
        let opened_lanes = self.open_lanes_for_remaining_parents(commit, commit_lane);
        for lane in &expecting {
            if *lane != commit_lane {
                self.set_expected(*lane, None);
            }
        }
        let lanes = self.row_spans(&occupied_above, &expecting, commit_lane, &opened_lanes);
        LaneRow {
            commit_lane: lane_number(commit_lane),
            lanes,
        }
    }

    fn lanes_expecting(&self, commit_id: &str) -> Vec<usize> {
        self.lanes
            .iter()
            .enumerate()
            .filter(|(_, lane)| lane.expected_id.as_deref() == Some(commit_id))
            .map(|(index, _)| index)
            .collect()
    }

    fn allocate_free_lane(&mut self) -> usize {
        match self
            .lanes
            .iter()
            .position(|lane| lane.expected_id.is_none())
        {
            Some(index) => index,
            None => {
                self.lanes.push(Lane {
                    expected_id: None,
                    color_slot: self.next_color_slot % LANE_COLOR_SLOT_COUNT,
                });
                self.next_color_slot = self.next_color_slot.wrapping_add(1);
                self.lanes.len() - 1
            }
        }
    }

    fn open_lanes_for_remaining_parents(
        &mut self,
        commit: &CommitSummary,
        commit_lane: usize,
    ) -> Vec<usize> {
        let mut parents = commit.parent_ids.iter();
        let Some(first_parent) = parents.next() else {
            self.set_expected(commit_lane, None);
            return Vec::new();
        };
        self.set_expected(commit_lane, Some(first_parent.clone()));
        let mut opened_lanes = Vec::new();
        for parent_id in parents {
            if !self.lanes_expecting(parent_id).is_empty() {
                continue;
            }
            let lane = self.allocate_free_lane();
            self.set_expected(lane, Some(parent_id.clone()));
            opened_lanes.push(lane);
        }
        opened_lanes
    }

    fn set_expected(&mut self, lane: usize, expected_id: Option<String>) {
        if let Some(lane) = self.lanes.get_mut(lane) {
            lane.expected_id = expected_id;
        }
    }

    fn row_spans(
        &self,
        occupied_above: &[bool],
        expecting: &[usize],
        commit_lane: usize,
        opened_lanes: &[usize],
    ) -> Vec<LaneSpan> {
        self.lanes
            .iter()
            .enumerate()
            .filter_map(|(index, lane)| {
                let continues_below = lane.expected_id.is_some();
                let continues_above = expecting.contains(&index)
                    || (!opened_lanes.contains(&index)
                        && occupied_above.get(index).copied().unwrap_or(false));
                let drawn = index == commit_lane || continues_above || continues_below;
                drawn.then(|| LaneSpan {
                    lane: lane_number(index),
                    continues_above,
                    continues_below,
                    color_slot: lane.color_slot,
                })
            })
            .collect()
    }
}

fn lane_number(lane: usize) -> u8 {
    u8::try_from(lane).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod graph_tests {
    use crate::repo::CommitSummary;

    use super::{assign_lanes, LaneRow, LANE_COLOR_SLOT_COUNT};

    fn commit(id: &str, parents: &[&str]) -> CommitSummary {
        CommitSummary {
            id: id.to_string(),
            short_id: id.to_string(),
            subject: format!("commit {id}"),
            author_name: String::from("gitdeck-fixture"),
            author_email: String::from("fixture@gitdeck.invalid"),
            author_timestamp: 1_577_836_800,
            committer_name: String::from("gitdeck-fixture"),
            committer_email: String::from("fixture@gitdeck.invalid"),
            committer_timestamp: 1_577_836_800,
            parent_ids: parents.iter().map(|parent| parent.to_string()).collect(),
            refs: Vec::new(),
        }
    }

    fn lanes_of(row: &LaneRow) -> Vec<(u8, bool, bool)> {
        row.lanes
            .iter()
            .map(|span| (span.lane, span.continues_above, span.continues_below))
            .collect()
    }

    fn color_slot_of(row: &LaneRow, lane: u8) -> Option<u8> {
        row.lanes
            .iter()
            .find(|span| span.lane == lane)
            .map(|span| span.color_slot)
    }

    fn octopus_history(parent_count: usize) -> Vec<CommitSummary> {
        let parent_ids: Vec<String> = (0..parent_count).map(|index| format!("p{index}")).collect();
        let parent_references: Vec<&str> = parent_ids.iter().map(String::as_str).collect();
        let mut history = vec![commit("m", &parent_references)];
        history.extend(parent_ids.iter().map(|parent| commit(parent, &[])));
        history
    }

    fn branching_history() -> Vec<CommitSummary> {
        vec![
            commit("m", &["a", "b"]),
            commit("b", &["r"]),
            commit("a", &["r"]),
            commit("r", &[]),
        ]
    }

    #[test]
    fn linear_history_uses_one_lane() {
        let history = vec![commit("c", &["b"]), commit("b", &["a"]), commit("a", &[])];
        let rows = assign_lanes(&history);
        assert_eq!(rows.len(), 3);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.commit_lane, 0, "row {index} commit lane");
            assert_eq!(
                lanes_of(row),
                vec![(0, index > 0, index < 2)],
                "row {index}"
            );
        }
    }

    #[test]
    fn branch_and_merge_create_two_lanes_and_a_merge_row() {
        let rows = assign_lanes(&branching_history());
        assert_eq!(lanes_of(&rows[0]), vec![(0, false, true), (1, false, true)]);
        assert_eq!(lanes_of(&rows[3]), vec![(0, true, false), (1, true, false)]);
        assert_eq!(rows[3].commit_lane, 0, "the merge row keeps the base lane");
        assert_eq!(
            color_slot_of(&rows[0], 1),
            Some(1),
            "the second lane carries the second color slot"
        );
    }

    #[test]
    fn octopus_merge_creates_lanes_for_each_parent() {
        let history = vec![
            commit("m", &["a", "b", "c"]),
            commit("a", &[]),
            commit("b", &[]),
            commit("c", &[]),
        ];
        let rows = assign_lanes(&history);
        assert_eq!(
            lanes_of(&rows[0]),
            vec![(0, false, true), (1, false, true), (2, false, true)]
        );
        assert_eq!(
            (0..3)
                .map(|lane| color_slot_of(&rows[0], lane))
                .collect::<Vec<Option<u8>>>(),
            vec![Some(0), Some(1), Some(2)]
        );
    }

    #[test]
    fn second_root_starts_a_new_lane() {
        let history = vec![
            commit("x", &["first-root"]),
            commit("second-root", &[]),
            commit("first-root", &[]),
        ];
        let rows = assign_lanes(&history);
        assert_eq!(
            rows[1].commit_lane, 1,
            "the unrelated root opens its own lane"
        );
        assert_eq!(lanes_of(&rows[1]), vec![(0, true, true), (1, false, false)]);
        assert_eq!(rows[2].commit_lane, 0);
    }

    #[test]
    fn lane_color_slots_are_stable_across_rows() {
        let rows = assign_lanes(&branching_history());
        for row in &rows {
            assert_eq!(color_slot_of(row, 0), Some(0), "lane 0 keeps its slot");
        }
        for row in &rows[1..3] {
            assert_eq!(color_slot_of(row, 1), Some(1), "lane 1 keeps its slot");
        }
    }

    #[test]
    fn meeting_lanes_keep_the_smaller_index() {
        let rows = assign_lanes(&branching_history());
        let meeting_row = &rows[3];
        assert_eq!(
            meeting_row.commit_lane, 0,
            "the smaller lane wins the meeting"
        );
        assert_eq!(
            lanes_of(meeting_row),
            vec![(0, true, false), (1, true, false)],
            "the larger lane ends on the meeting row"
        );
    }

    #[test]
    fn empty_input_produces_no_rows() {
        assert!(assign_lanes(&[]).is_empty());
    }

    #[test]
    fn lane_index_never_exceeds_the_visible_width() {
        let rows = assign_lanes(&octopus_history(5));
        assert_eq!(
            lanes_of(&rows[0]).len(),
            5,
            "every parent keeps its own lane even beyond the three visible ones"
        );
        assert_eq!(lanes_of(&rows[0])[4], (4, false, true));
        assert_eq!(color_slot_of(&rows[0], 4), Some(4));
    }

    #[test]
    fn color_slots_cycle_after_six_lanes() {
        let rows = assign_lanes(&octopus_history(8));
        assert_eq!(color_slot_of(&rows[0], 6), Some(0));
        assert_eq!(color_slot_of(&rows[0], 7), Some(1));
        assert_eq!(LANE_COLOR_SLOT_COUNT, 6);
    }
}
