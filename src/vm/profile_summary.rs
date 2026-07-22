pub mod slot_view_model {
    use crate::save::common::user_data_10::ProfileSummary;


    #[derive(Clone)]
    #[derive(Default)]
    pub struct ProfileSummaryViewModel {
        pub active: bool,
        /// Unread: callers take the name from `GeneralViewModel`. Kept because the
        /// profile summary is its own structure in the save and does carry a name.
        #[allow(dead_code)]
        pub character_name: String,
    }

    
    
    impl ProfileSummaryViewModel {
        pub fn from_save(profile_summary: &ProfileSummary) -> Self {
            let active = true;
            
            // Character Name
            let character_name = profile_summary.character_name;
            let character_name = String::from_utf16(&character_name).expect("");
             
            Self {
                active,
                character_name
            }
        }
    }
}