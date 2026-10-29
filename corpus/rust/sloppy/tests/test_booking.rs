// Calls every booking function except hold. hold's lines stay unhit.
fn books_a_room() {
    let mut guest = Request {
        weekend: true,
        holiday: false,
        suite: true,
        late: false,
        early: false,
        group: false,
        member: true,
        quiet: false,
        high_floor: false,
        view: false,
        parking: false,
        breakfast: false,
        crib: false,
        accessible: false,
        prepaid: false,
        invoice: false,
        long_stay: false,
        walk_in: false,
        corporate: false,
        government: false,
        student: false,
        military: false,
        repeat_guest: false,
        connecting: false,
        guest: true,
        night: true,
        room: true,
        paid: true,
        loyalty: true,
        policy: true,
        double_room: false,
        single: false,
        card: false,
        kind: 0,
        member_rate: 40,
        rack_rate: 80,
    };
    assert!(choose(&guest));
    assert_eq!(arrange(&mut guest), 1);
    assert_eq!(forward("ada"), "ada");
    let confirmation = Confirmation {
        stay: Stay {
            room: Room {
                floor: Floor {
                    wing: Wing {
                        desk: Desk { label: 1 },
                    },
                },
            },
        },
    };
    assert_eq!(city(&confirmation), 1);
    assert_eq!(
        book(
            "ada",
            "ada@example.com",
            "555-0100",
            2,
            3,
            "Friday",
            "Monday",
            400,
            true,
            true,
            6,
            true,
            "card",
            "Northwind",
            "late arrival",
            true,
            true,
            false,
            true,
            false,
        ),
        "ada"
    );
    assert!(describe(true));
    assert_eq!(record(), 72);
}
