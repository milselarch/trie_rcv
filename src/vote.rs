use std::collections::HashSet;

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
    NonFinalSpecialVote,
    DuplicateVotes,
    VoteIsEmpty
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
    pub const fn to_int(&self) -> i32 {
        match self {
            SpecialVotes::WITHHOLD => -1,
            SpecialVotes::ABSTAIN => -2
        }
    }

    pub const fn from_int(raw_value: i32) -> Result<SpecialVotes, VoteErrors> {
        match raw_value {
            -1 => Ok(SpecialVotes::WITHHOLD),
            -2 => Ok(SpecialVotes::ABSTAIN),
            _ => Err(VoteErrors::InvalidCastToSpecialVote)
        }
    }
}

pub struct RankedVote {
    rankings: Vec<u16>,
    special_vote: Option<SpecialVotes>
}

trait Vote {
    fn to_vector(&self) -> Vec<i32>;
}

impl RankedVote {
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

    pub fn from_vectors(
        raw_votes: &Vec<Vec<i32>>
    ) -> Result<Vec<RankedVote>, VoteErrors> {
        let mut votes: Vec<RankedVote> = Vec::new();

        for raw_vote in raw_votes {
            let result = RankedVote::from_vector(raw_vote);
            match result {
                Err(err) => return Err(err),
                Ok(vote_struct) => {
                    votes.push(vote_struct)
                }
            }
        }

        return Ok(votes);
    }

    pub fn from_candidates(
        candidates: &Vec<u16>
    ) -> Result<RankedVote, VoteErrors> {
        return Self::from_vector(
            &candidates.iter().map(|x| *x as i32).collect()
        )
    }

    pub fn from_vector(
        raw_ranked_vote: &Vec<i32>
    ) -> Result<RankedVote, VoteErrors> {
        // println!("INSERT {:?}", raw_rankings);
        let mut candidates: Vec<u16> = Vec::new();
        let mut special_vote_value: Option<SpecialVotes> = None;
        let mut unique_values = HashSet::new();
        let length = raw_ranked_vote.len();
        let last_index = length - 1;

        for (k, raw_ranked_vote_value) in raw_ranked_vote.iter().enumerate() {
            let is_last_index = k == last_index;

            if unique_values.contains(raw_ranked_vote_value) {
                return Err(VoteErrors::DuplicateVotes);
            } else {
                unique_values.insert(*raw_ranked_vote_value);
            }

            if raw_ranked_vote_value.is_negative() {
                if !is_last_index {
                    return Err(VoteErrors::NonFinalSpecialVote);
                }
                assert!(is_last_index);
                let cast_result =
                    SpecialVotes::from_int(*raw_ranked_vote_value);
                match cast_result {
                    Err(cast_error) => { return Err(cast_error); },
                    Ok(cast_value) => {
                        special_vote_value = Some(cast_value)
                    }
                }
            } else {
                assert!(raw_ranked_vote_value.is_positive());
                let cast_result = u16::try_from(*raw_ranked_vote_value);
                match cast_result {
                    Ok(candidate) => { candidates.push(candidate) }
                    Err(_) => {
                        return Err(VoteErrors::InvalidCastToSpecialVote);
                    },
                }
            }
        }

        if special_vote_value.is_none() && candidates.is_empty() {
            return Err(VoteErrors::VoteIsEmpty)
        }

        // println!("INSERT_END {:?}", raw_rankings);
        return Ok(RankedVote {
            rankings: candidates, special_vote: special_vote_value
        })
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
                let item = Some(VoteValues::SpecialVote(*special_vote));
                self.special_vote = None;
                return item;
            }
        }
    }
}

impl RankedVote {
    // Method to create an iterator over the vote values
    pub fn iter(&self) -> VoteStructIterator {
        VoteStructIterator {
            rankings_iter: self.rankings.iter(),
            special_vote: self.special_vote.as_ref(),
        }
    }
}

pub trait ToVotes {
    fn to_votes(&self) -> Result<Vec<RankedVote>, VoteErrors>;
}

impl ToVotes for Vec<Vec<i32>> {
    fn to_votes(&self) -> Result<Vec<RankedVote>, VoteErrors> {
        return RankedVote::from_vectors(self);
    }
}