use csustkit::{
    campus_card::{Campus, CampusCardHelper},
    sso::SsoHelper,
};

use super::console;

pub async fn run_menu(helper: &CampusCardHelper, sso: &SsoHelper) {
    let ticket = match sso.login_to_campus_card().await {
        Ok(ticket) => ticket,
        Err(error) => {
            println!("进入校园卡系统失败: {error}");
            return;
        }
    };
    if let Err(error) = helper.sync_token(ticket).await {
        println!("无法获取校园卡 token: {error}");
        return;
    }
    loop {
        println!("\n=== 宿舍电量查询 ===");
        println!("1. 获取用户信息");
        println!("2. 查询电量");
        println!("3. 退出校园卡系统");
        println!("0. 返回上一级");
        let Some(choice) = console::prompt("请选择") else {
            return;
        };
        match choice.as_str() {
            "1" => match helper.get_profile().await {
                Ok(profile) => {
                    println!("\n姓名: {}", profile.name);
                    println!("学号: {}", profile.sno);
                    println!("院系: {}", profile.department_name);
                    println!("宿舍班级: {}", profile.class_name);
                }
                Err(error) => println!("操作失败: {error}"),
            },
            "2" => {
                if let Err(error) = query_electricity(helper).await {
                    println!("操作失败: {error}");
                }
            }
            "3" => match helper.logout().await {
                Ok(()) => println!("校园卡系统已退出。"),
                Err(error) => println!("退出校园卡系统失败: {error}"),
            },
            "0" => return,
            _ => println!("输入无效，请重新选择。"),
        }
    }
}

async fn query_electricity(
    helper: &CampusCardHelper,
) -> Result<(), csustkit::campus_card::CampusCardError> {
    let Some(campus) = console::select_value(
        "请选择校区",
        &[(Campus::Yuntang, "云塘"), (Campus::Jinpenling, "金盆岭")],
    ) else {
        return Ok(());
    };
    let mut buildings = helper.get_buildings(campus).await?;
    buildings.sort_by(|left, right| left.name.cmp(&right.name));
    if buildings.is_empty() {
        println!("{}楼栋列表为空。", campus.display_name());
        return Ok(());
    }
    let Some(building) =
        console::select("楼栋列表", &buildings, |building| building.name.clone())
    else {
        return Ok(());
    };
    let mut rooms = helper.get_rooms(building).await?;
    rooms.sort_by(|left, right| left.name.cmp(&right.name));
    if rooms.is_empty() {
        println!("{}宿舍列表为空。", building.name);
        return Ok(());
    }
    let Some(room) = console::select("宿舍列表", &rooms, |room| room.name.clone()) else {
        return Ok(());
    };
    let electricity = helper.get_electricity(room).await?;
    println!("\n查询结果:");
    println!("校区: {}", campus.display_name());
    println!("楼栋: {}", building.name);
    println!("宿舍号: {}", room.name);
    println!("剩余电量: {electricity:.2} 度");
    Ok(())
}
