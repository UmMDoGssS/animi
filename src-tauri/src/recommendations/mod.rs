#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecommendationWeights {
    pub tags: f32,
    pub genres: f32,
    pub trending: f32,
}

impl RecommendationWeights {
    pub const V1: Self = Self {
        tags: 0.55,
        genres: 0.25,
        trending: 0.20,
    };

    pub fn total(self) -> f32 {
        self.tags + self.genres + self.trending
    }
}

#[cfg(test)]
mod tests {
    use super::RecommendationWeights;

    #[test]
    fn project_weights_form_a_complete_score() {
        let difference = (RecommendationWeights::V1.total() - 1.0).abs();
        assert!(difference < f32::EPSILON);
    }
}
