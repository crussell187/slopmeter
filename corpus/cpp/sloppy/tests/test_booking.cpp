// Calls every booking function except hold. hold's lines stay unhit.
int books_a_room() {
    Request guest = {};
    guest.weekend = true;
    guest.suite = true;
    guest.guest = true;
    guest.night = true;
    guest.room = true;
    guest.paid = true;
    guest.loyalty = true;
    guest.policy = true;
    if (!choose(guest)) {
        return 1;
    }
    if (arrange(guest) != 1) {
        return 1;
    }
    if (forward("ada")[0] != 'a') {
        return 1;
    }
    Confirmation confirmation = {};
    confirmation.stay.room.floor.wing.desk.label = 1;
    if (city(confirmation) != 1) {
        return 1;
    }
    if (book(
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
            false)[0] != 'a') {
        return 1;
    }
    if (!describe(true)) {
        return 1;
    }
    if (record() != 72) {
        return 1;
    }
    return 0;
}
