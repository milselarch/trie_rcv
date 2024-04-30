#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpecialVotes {
    WITHHOLD,
    ABSTAIN
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum VoteValues {
    Candidate(u16),
    SpecialVote(SpecialVotes)
}

#[derive(Debug)]
pub enum VoteErrors {
    InvalidCastToSpecialVote,
    ReadOutOfBounds,
    NonFinalSpecialVote
}

impl VoteValues {
    fn to_int(&self) -> i32 {
        match self {
            VoteValues::Candidate(choice) => { i32::from(*choice) }
            VoteValues::SpecialVote(special_vote) => { special_vote.to_int() }
        }
    }

    fn from_int(raw_value: i32) -> Result<VoteValues, VoteErrors> {
        let cast_result = u16::try_from(raw_value);

        return match cast_result {
            Err(_) => { Err(VoteErrors::InvalidCastToSpecialVote) },
            Ok(value) => { Ok(VoteValues::Candidate(value)) }
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

    fn from_int(raw_value: i32) -> Result<SpecialVotes, VoteErrors> {
        match raw_value {
            -1 => Ok(SpecialVotes::WITHHOLD),
            -2 => Ok(SpecialVotes::ABSTAIN),
            _ => Err(VoteErrors::InvalidCastToSpecialVote)
        }
    }
}

pub struct VoteStruct {
    rankings: Vec<u16>,
    special_vote: Option<SpecialVotes>
}

trait Vote {
    fn to_vector(&self) -> Vec<i32>;
}

impl VoteStruct {
    pub fn len(&self) -> usize {
        let mut length = self.rankings.len();
        if self.special_vote.is_some() { length += 1; }
        return length;
    }

    pub fn get(&self, index: usize) -> Result<VoteValues, VoteErrors> {
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
            None => { Err(VoteErrors::ReadOutOfBounds) }
            Some(choice) => { Ok(VoteValues::Candidate(*choice)) }
        }
    }

    pub fn from_vector(raw_rankings: Vec<i32>) -> Result<VoteStruct, VoteErrors> {
        let mut rankings: Vec<u16> = Vec::new();
        let mut special_vote: Option<SpecialVotes> = None;
        let length = raw_rankings.len();
        let last_index = length - 1;

        for (k, raw_ranking) in raw_rankings.iter().enumerate() {
            let is_last_index = k == last_index;
            if raw_ranking.is_negative() {
                if !is_last_index {
                    return Err(VoteErrors::NonFinalSpecialVote);
                }
                assert!(is_last_index);
                let cast_result = SpecialVotes::from_int(*raw_ranking);
                match cast_result {
                    Err(cast_error) => { return Err(cast_error); },
                    Ok(cast_value) => { special_vote = Some(cast_value) }
                }
            } else {
                assert!(raw_ranking.is_positive());
                let cast_result = u16::try_from(*raw_ranking);
                match cast_result {
                    Err(_) => { return Err(VoteErrors::InvalidCastToSpecialVote); },
                    Ok(choice) => { rankings.push(choice) }
                }
            }
        }

        return Ok(VoteStruct { rankings, special_vote })
    }

    pub fn to_vector(&self) -> Vec<i32> {
        let mut all_rankings: Vec<i32> = Vec::new();
        for ranking in &self.rankings {
            all_rankings.push(i32::from(*ranking));
        }
        if let Some(special_vote) = &self.special_vote {
            all_rankings.push(special_vote.to_int())
        }
        return all_rankings;
    }
}

pub struct VoteStructIterator<'a> {
    rankings_iter: std::slice::Iter<'a, u16>,
    special_vote: Option<&'a SpecialVotes>,
}

impl<'a> Iterator for VoteStructIterator<'a> {
    type Item = VoteValues;

    fn next(&mut self) -> Option<Self::Item> {
        // create iterator for normal rankings
        let ranking = self.rankings_iter.next().map(
            |&r| VoteValues::Candidate(r)
        );
        if ranking.is_some() {
            return ranking;
        }

        // return special vote last
        return match self.special_vote {
            None => None,
            Some(special_vote) => {
                Some(VoteValues::SpecialVote(*special_vote))
            }
        }
    }
}

impl VoteStruct {
    // Method to create an iterator over the vote values
    pub fn iter(&self) -> VoteStructIterator {
        VoteStructIterator {
            rankings_iter: self.rankings.iter(),
            special_vote: self.special_vote.as_ref(),
        }
    }
}