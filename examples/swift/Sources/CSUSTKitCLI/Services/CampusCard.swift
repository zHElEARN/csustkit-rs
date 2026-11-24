import CSUSTKitRust
import Foundation

@MainActor
enum CampusCardMenu {
    private enum Action {
        case profile, electricity, logout
    }

    static func run(_ console: Console, helper: CampusCardHelper, sso: SsoHelper) async {
        do {
            let ticket = try await sso.loginToCampusCard()
            try await helper.syncToken(ticket: ticket)
        } catch {
            Console.report("进入校园卡系统失败", error)
            return
        }

        let actions: [(Action, String)] = [
            (.profile, "查看个人信息"),
            (.electricity, "查询宿舍电量"),
            (.logout, "登出校园卡"),
        ]
        while let action = console.menu("校园卡", actions) {
            do {
                switch action {
                case .profile:
                    let profile = try await helper.getProfile()
                    print("姓名：\(profile.name)")
                    print("学号：\(profile.sno)")
                    print("院系：\(profile.departmentName)")
                    print("宿舍班级：\(profile.className)")
                case .electricity:
                    try await electricity(console, helper: helper)
                case .logout:
                    try await helper.logout()
                    print("已登出校园卡。")
                    return
                }
            } catch {
                Console.report(action == .logout ? "登出校园卡失败" : "操作失败", error)
            }
        }
    }

    private static func electricity(_ console: Console, helper: CampusCardHelper) async throws {
        guard let campus = ServiceDisplay.campus(console) else { return }

        let buildings = try await helper.getBuildings(campus: campus).sorted { $0.name < $1.name }
        if buildings.isEmpty {
            print("\(campus.displayName())楼栋列表为空。")
            return
        }
        guard let building = console.select("选择楼栋", buildings, label: \.name) else { return }

        let rooms = try await helper.getRooms(building: building).sorted { $0.name < $1.name }
        if rooms.isEmpty {
            print("\(building.name)宿舍列表为空。")
            return
        }
        guard let room = console.select("选择宿舍", rooms, label: \.name) else { return }

        let amount = try await helper.getElectricity(room: room)
        let electricity = String(format: "%.2f", locale: Locale(identifier: "en_US_POSIX"), amount)
        print("\(campus.displayName()) | \(building.name) | \(room.name) | 剩余电量 \(electricity) 度")
    }
}
