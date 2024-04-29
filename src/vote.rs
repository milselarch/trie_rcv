enum SpecialVotes {
    WITHHOLD,
    ABSTAIN
}

enum VoteValues {
    Choice(u16),
    SpecialVote(SpecialVotes)
}

impl VoteValues {
    fn to_int(&self) -> i32 {
        match self {
            VoteValues::Choice(choice) => { i32::from(choice) }
            VoteValues::SpecialVote(special_vote) => { special_vote.to_int() }
        }
    }

    fn from_int(raw_value: i32) -> Result<VoteValues, Err> {
        return if (raw_value.is_positive()) {
            Ok(VoteValues::Choice(u16::from(raw_value)))
        } else {
            let special_vote_result = SpecialVotes::from_int(raw_value);
            match special_vote_result {
                Err(cast_error) => { Err(cast_error) }
                Some(special_vote) => {
                    Ok(VoteValues::SpecialVote(special_vote))
                }
            }
        }
    }
}

impl SpecialVotes {
    fn to_int(&self) -> i32 {
        match self {
            SpecialVotes::WITHHOLD => -1,
            SpecialVotes::ABSTAIN => -2
        }
    }

    fn from_int(raw_value: i32) -> Result<SpecialVotes, Err> {
        match raw_value {
            -1 => Ok(SpecialVotes::WITHHOLD),
            -2 => Ok(SpecialVotes::ABSTAIN),
            _ => Err("Invalid input Value")
        }
    }
}

struct VoteStruct {
    rankings: Vec<u16>,
    special_vote: Option<SpecialVotes>
}

trait Vote {
    fn to_vector(&self) -> Vec<i32>;
}

impl VoteStruct {
    fn len(&self) -> usize {
        let mut length = self.rankings.len();
        if self.special_vote.is_some() { length += 1; }
        return length;
    }

    fn get(&self, index: usize) -> Result<VoteValues, Err> {
        let rankings_length = self.rankings.len();
        let special_vote_option = self.special_vote.clone();

        if index == rankings_length {
            match special_vote_option {
                Some(special_vote) => {
                    return Ok(VoteValues::SpecialVote(special_vote))
                }
                _ => {}
            }
        }

        let read_result = self.rankings.get(index);
        match read_result {
            Err(read_error) => { Err(read_error) }
            Some(choice) => { Ok(VoteValues::Choice(*choice)) }
        }
    }

    fn from_vector(raw_rankings: Vec<i32>) -> Result<VoteStruct, Err> {
        let mut rankings: Vec<u16> = Vec::new();
        let mut special_vote: Option<SpecialVotes> = None;
        let length = raw_rankings.len();
        let last_index = length - 1;

        for (k, raw_ranking) in raw_rankings.iter().enumerate() {
            let is_last_index = k == last_index;
            if raw_ranking.is_negative() {
                if (!is_last_index) {
                    return Err("Only last vote can have negative Value");
                }

                assert!(is_last_index);
                let cast_result = SpecialVotes::from_int(*raw_ranking);
                match cast_result {
                    Err (cast_error) => { return Err(cast_error); },
                    Some (cast_value) => { special_vote = cast_value }
                }
            } else {
                assert!(raw_ranking.is_positive());
                let cast_value = u16::from(raw_ranking);
                rankings.push(cast_value);
            }
        }

        return VoteStruct::new(rankings, special_vote);
    }
}

impl Vote for VoteStruct {
    fn to_vector(&self) -> Vec<i32> {
        let mut all_rankings: Vec<i32> = Vec::new();
        for ranking in self.rankings {
            all_rankings.push(i32::from(ranking));
        }

        if let Some(special_vote) = &self.special_vote {
            match special_vote {
                SpecialVotes::WITHHOLD => all_rankings.push(-1),
                SpecialVotes::ABSTAIN => all_rankings.push(-2),
            }
        }

        return all_rankings;
    }
}