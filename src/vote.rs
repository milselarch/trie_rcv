use std::collections::HashSet;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
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
    InvalidCastToCandidate,
    InvalidCastToSpecialVote,
    ReadOutOfBounds,
    NonFinalSpecialVote,
    DuplicateVotes,
    VoteIsEmpty
}

impl VoteValues {
    pub fn to_int(self) -> i32 {
        match self {
            VoteValues::Candidate(choice) => { i32::from(choice) }
            VoteValues::SpecialVote(special_vote) => { special_vote.to_int() }
        }
    }

    pub fn from_int(raw_value: i32) -> Result<VoteValues, VoteErrors> {
        let special_vote_cast_result = SpecialVotes::from_int(raw_value);
        if let Ok(special_vote) = special_vote_cast_result {
            return Ok(VoteValues::SpecialVote(special_vote));
        }

        let cast_result = u16::try_from(raw_value);

        match cast_result {
            Err(_) => { Err(VoteErrors::InvalidCastToCandidate) },
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

impl RankedVote {
    pub fn len(&self) -> usize {
        let mut length = self.rankings.len();
        if self.special_vote.is_some() { length += 1; }
        length
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Result<VoteValues, VoteErrors> {
        let rankings_length = self.rankings.len();
        let special_vote_option = self.special_vote;

        if index == rankings_length {
            if let Some(special_vote) = special_vote_option {
                return Ok(VoteValues::SpecialVote(special_vote))
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

        Ok(votes)
    }

    pub fn from_candidates(
        candidates: &[u16]
    ) -> Result<RankedVote, VoteErrors> {
        return Self::from_vector(
            &candidates.iter().map(|x| *x as i32).collect()
        )
    }

    #[allow(clippy::ptr_arg)]
    pub fn from_vector(
        raw_ranked_vote: &Vec<i32>
    ) -> Result<RankedVote, VoteErrors> {
        // println!("INSERT {:?}", raw_rankings);
        let mut candidates: Vec<u16> = Vec::new();
        let mut special_vote_value: Option<SpecialVotes> = None;
        let mut unique_values = HashSet::new();

        for (k, raw_ranked_vote_value) in raw_ranked_vote.iter().enumerate() {
            let length = raw_ranked_vote.len();
            let last_index = length - 1;
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
        Ok(RankedVote {
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
        all_rankings
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
        match self.special_vote {
            None => None,
            Some(special_vote) => {
                let item = Some(VoteValues::SpecialVote(*special_vote));
                self.special_vote = None;
                item
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
        RankedVote::from_vectors(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_vote_not_allowed() {
        let cast_result = RankedVote::from_vector(&vec![]);
        assert!(cast_result.is_err());
    }

    #[test]
    fn test_duplicate_vote_not_allowed() {
        let cast_result = RankedVote::from_vector(&vec![1, 2, 1]);
        assert!(cast_result.is_err());
    }

    #[test]
    fn test_special_vote_enum_consistency() {
        let withhold_vote_val =
            SpecialVotes::from_int(SpecialVotes::WITHHOLD.to_int()).unwrap();
        let abstain_vote_val =
            SpecialVotes::from_int(SpecialVotes::ABSTAIN.to_int()).unwrap();

        assert_ne!(withhold_vote_val, abstain_vote_val);
        assert_eq!(withhold_vote_val, SpecialVotes::WITHHOLD);
        assert_eq!(abstain_vote_val, SpecialVotes::ABSTAIN);
    }

    #[test]
    fn test_non_final_special_votes_not_allowed() {
        let cast_result = RankedVote::from_vector(&vec![
            1, 2, 4, SpecialVotes::WITHHOLD.to_int(), 1
        ]);
        assert!(cast_result.is_err());
    }

    #[test]
    fn test_from_to_vector() {
        // checks from then to vector conversion yields original input
        let raw_ranked_vote = vec![1, 2, 6, 3];
        assert_eq!(
            RankedVote::from_vector(&raw_ranked_vote).unwrap().to_vector(),
            raw_ranked_vote
        )
    }
}