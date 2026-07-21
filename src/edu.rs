mod helper;
pub(crate) mod request;
mod types;

pub use helper::EduHelper;
pub use types::{
    Course, CourseGrade, CourseGradeQuery, CourseNature, CourseSchedule, DayOfWeek, DisplayMode,
    EduError, EducationRequestError, Exam, ExamScheduleQuery, GradeComponent, GradeDetail, Profile,
    ScheduleSession, SemesterOptions, SemesterType, StudyMode,
};
