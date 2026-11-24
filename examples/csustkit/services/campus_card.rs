use csustkit::{
    campus_card::{CampusCardError, CampusCardHelper},
    sso::SsoHelper,
};

use super::{Console, choose_campus};

#[derive(Clone, Copy)]
enum Action {
    Profile,
    Electricity,
    Logout,
}

pub async fn run_menu(console: &mut Console, helper: &CampusCardHelper, sso: &SsoHelper) {
    let ticket = match sso.login_to_campus_card().await {
        Ok(ticket) => ticket,
        Err(error) => {
            eprintln!("进入校园卡系统失败：{error}");
            return;
        }
    };
    if let Err(error) = helper.sync_token(ticket).await {
        eprintln!("无法获取校园卡 token：{error}");
        return;
    }

    while let Some(action) = console.menu(
        "校园卡",
        &[
            (Action::Profile, "查看个人信息"),
            (Action::Electricity, "查询宿舍电量"),
            (Action::Logout, "登出校园卡"),
        ],
        "返回",
    ) {
        let result = match action {
            Action::Profile => profile(helper).await,
            Action::Electricity => query_electricity(console, helper).await,
            Action::Logout => match helper.logout().await {
                Ok(()) => {
                    println!("已登出校园卡。");
                    return;
                }
                Err(error) => {
                    eprintln!("登出校园卡失败：{error}");
                    continue;
                }
            },
        };
        if let Err(error) = result {
            eprintln!("操作失败：{error}");
        }
    }
}

async fn profile(helper: &CampusCardHelper) -> Result<(), CampusCardError> {
    let profile = helper.get_profile().await?;
    println!("姓名：{}", profile.name);
    println!("学号：{}", profile.sno);
    println!("院系：{}", profile.department_name);
    println!("宿舍班级：{}", profile.class_name);
    Ok(())
}

async fn query_electricity(
    console: &mut Console,
    helper: &CampusCardHelper,
) -> Result<(), CampusCardError> {
    let Some(campus) = choose_campus(console) else {
        return Ok(());
    };
    let mut buildings = helper.get_buildings(campus).await?;
    buildings.sort_by(|left, right| left.name.cmp(&right.name));
    if buildings.is_empty() {
        println!("{}楼栋列表为空。", campus.display_name());
        return Ok(());
    }
    let Some(building) = console.select("选择楼栋", &buildings, |building| &building.name)
    else {
        return Ok(());
    };
    let mut rooms = helper.get_rooms(building).await?;
    rooms.sort_by(|left, right| left.name.cmp(&right.name));
    if rooms.is_empty() {
        println!("{}宿舍列表为空。", building.name);
        return Ok(());
    }
    let Some(room) = console.select("选择宿舍", &rooms, |room| &room.name) else {
        return Ok(());
    };

    let electricity = helper.get_electricity(room).await?;
    println!(
        "{} | {} | {} | 剩余电量 {electricity:.2} 度",
        campus.display_name(),
        building.name,
        room.name
    );
    Ok(())
}
