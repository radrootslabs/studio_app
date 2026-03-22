use crate::{RadrootsLocationPoint, RadrootsResolvedLocation};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HomeLocationLookupResult {
    pub queried_point: RadrootsLocationPoint,
    pub matches: Vec<RadrootsResolvedLocation>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct HomeLocationTools {
    latitude_input: String,
    longitude_input: String,
    status_message: Option<String>,
    lookup_result: Option<HomeLocationLookupResult>,
}

impl HomeLocationTools {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn clear(&mut self) {
        self.latitude_input.clear();
        self.longitude_input.clear();
        self.status_message = None;
        self.lookup_result = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_resets_inputs_and_feedback() {
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = "10.5".to_owned();
        tools.longitude_input = "20.5".to_owned();
        tools.status_message = Some("lookup failed".to_owned());
        tools.lookup_result = Some(HomeLocationLookupResult {
            queried_point: RadrootsLocationPoint {
                lat: 10.5,
                lng: 20.5,
            },
            matches: Vec::new(),
        });

        tools.clear();

        assert_eq!(tools.latitude_input, "");
        assert_eq!(tools.longitude_input, "");
        assert_eq!(tools.status_message, None);
        assert_eq!(tools.lookup_result, None);
    }
}
