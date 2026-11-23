use crate::{
    Campus,
    chaoxing::AssignmentStatus,
    edu::{CourseNature, DisplayMode, SemesterType, StudyMode},
    sso::SsoProfile,
};

#[uniffi::export]
impl Campus {
    #[uniffi::method(name = "short_name")]
    fn short_name_string(&self) -> String {
        self.short_name().to_owned()
    }

    #[uniffi::method(name = "display_name")]
    fn display_name_string(&self) -> String {
        self.display_name().to_owned()
    }
}

#[uniffi::export]
fn campus_from_name(name: &str) -> Option<Campus> {
    Campus::from_name(name)
}

#[uniffi::export]
impl SemesterType {
    #[uniffi::method(name = "display_name")]
    fn display_name_string(&self) -> String {
        self.display_name().to_owned()
    }
}

#[uniffi::export]
impl DisplayMode {
    #[uniffi::method(name = "display_name")]
    fn display_name_string(&self) -> String {
        self.display_name().to_owned()
    }
}

#[uniffi::export]
impl StudyMode {
    #[uniffi::method(name = "display_name")]
    fn display_name_string(&self) -> String {
        self.display_name().to_owned()
    }
}

#[uniffi::export]
impl CourseNature {
    #[uniffi::method(name = "display_name")]
    fn display_name_string(&self) -> String {
        self.display_name().to_owned()
    }
}

#[uniffi::export(name = "ChaoxingAssignmentStatus")]
impl AssignmentStatus {
    #[uniffi::method(name = "display_name")]
    fn display_name_string(&self) -> String {
        self.display_name().to_owned()
    }
}

#[uniffi::export]
impl SsoProfile {
    #[uniffi::method(name = "avatar_url")]
    fn avatar_url_string(&self) -> String {
        self.avatar_url().to_owned()
    }
}
