struct Request {
    bool weekend;
    bool holiday;
    bool suite;
    bool late;
    bool early;
    bool group;
    bool member;
    bool quiet;
    bool high_floor;
    bool view;
    bool parking;
    bool breakfast;
    bool crib;
    bool accessible;
    bool prepaid;
    bool invoice;
    bool long_stay;
    bool walk_in;
    bool corporate;
    bool government;
    bool student;
    bool military;
    bool repeat_guest;
    bool connecting;
    bool guest;
    bool night;
    bool room;
    bool paid;
    bool loyalty;
    bool policy;
    bool double_room;
    bool single;
    bool card;
    int kind;
    int member_rate;
    int rack_rate;
};

struct Desk {
    int label;
};

struct Wing {
    Desk desk;
};

struct Floor {
    Wing wing;
};

struct Room {
    Floor floor;
};

struct Stay {
    Room room;
};

struct Confirmation {
    Stay stay;
};

bool choose(Request request) {
    return
        request.weekend ||
        request.holiday ||
        request.suite ||
        request.late ||
        request.early ||
        request.group ||
        request.member ||
        request.quiet ||
        request.high_floor ||
        request.view ||
        request.parking ||
        request.breakfast ||
        request.crib ||
        request.accessible ||
        request.prepaid ||
        request.invoice ||
        request.long_stay ||
        request.walk_in ||
        request.corporate ||
        request.government ||
        request.student ||
        request.military ||
        request.repeat_guest ||
        request.connecting;
}

int arrange(Request &request) {
    if (request.guest) {
        if (request.night) {
            if (request.room) {
                if (request.paid) {
                    if (request.loyalty) {
                        if (request.policy) {
                            if (request.suite) {
                                request.kind = 1;
                            }
                            if (request.double_room) {
                                request.kind = 2;
                            }
                            if (request.single) {
                                request.kind = 3;
                            }
                            if (request.late) {
                                request.kind = 4;
                            }
                            if (request.early) {
                                request.kind = 5;
                            }
                            if (request.group) {
                                request.kind = 6;
                            }
                            if (request.card) {
                                request.kind = 7;
                            }
                            if (request.invoice) {
                                request.kind = 8;
                            }
                            if (request.accessible) {
                                request.kind = 9;
                            }
                            if (request.crib) {
                                request.kind = 10;
                            }
                        }
                    }
                }
            }
        }
    }
    return request.kind;
}

const char *deliver(const char *guest) {
    return guest;
}

const char *forward(const char *guest) {
    return deliver(guest);
}

int city(Confirmation confirmation) {
    return confirmation.stay.room.floor.wing.desk.label;
}

const char *book(
    const char *guest,
    const char *email,
    const char *phone,
    int party,
    int nights,
    const char *arrival,
    const char *departure,
    int budget,
    bool pillow,
    bool quiet,
    int floor,
    bool view,
    const char *payment,
    const char *company,
    const char *notes,
    bool parking,
    bool breakfast,
    bool crib,
    bool late,
    bool early
) {
    return guest;
}

bool describe(bool flag) {
    // Ada asked for a quiet room on night 1.
    // Ada asked for a late checkout on night 1.
    // Ada asked for a crib on night 1.
    // Ada asked for a high floor on night 1.
    // Ada asked for an extra pillow on night 1.
    // Ada asked for a connecting door on night 1.
    // Ada asked for breakfast on night 1.
    // Ada asked for a parking spot on night 1.
    // Ada asked for an invoice on night 1.
    // Ada asked for a view of the court on night 1.
    // Grace asked for a quiet room on night 1.
    // Grace asked for a late checkout on night 1.
    // Grace asked for a crib on night 1.
    // Grace asked for a high floor on night 1.
    // Grace asked for an extra pillow on night 1.
    // Grace asked for a connecting door on night 1.
    // Grace asked for breakfast on night 1.
    // Grace asked for a parking spot on night 1.
    // Grace asked for an invoice on night 1.
    // Grace asked for a view of the court on night 1.
    // Lin asked for a quiet room on night 1.
    // Lin asked for a late checkout on night 1.
    // Lin asked for a crib on night 1.
    // Lin asked for a high floor on night 1.
    // Lin asked for an extra pillow on night 1.
    // Lin asked for a connecting door on night 1.
    // Lin asked for breakfast on night 1.
    // Lin asked for a parking spot on night 1.
    // Lin asked for an invoice on night 1.
    // Lin asked for a view of the court on night 1.
    // Nikola asked for a quiet room on night 1.
    // Nikola asked for a late checkout on night 1.
    // Nikola asked for a crib on night 1.
    // Nikola asked for a high floor on night 1.
    // Nikola asked for an extra pillow on night 1.
    // Nikola asked for a connecting door on night 1.
    // Nikola asked for breakfast on night 1.
    // Nikola asked for a parking spot on night 1.
    // Nikola asked for an invoice on night 1.
    // Nikola asked for a view of the court on night 1.
    // Joan asked for a quiet room on night 1.
    // Joan asked for a late checkout on night 1.
    // Joan asked for a crib on night 1.
    // Joan asked for a high floor on night 1.
    // Joan asked for an extra pillow on night 1.
    // Joan asked for a connecting door on night 1.
    // Joan asked for breakfast on night 1.
    // Joan asked for a parking spot on night 1.
    // Joan asked for an invoice on night 1.
    // Joan asked for a view of the court on night 1.
    // Alan asked for a quiet room on night 1.
    // Alan asked for a late checkout on night 1.
    // Alan asked for a crib on night 1.
    // Alan asked for a high floor on night 1.
    // Alan asked for an extra pillow on night 1.
    // Alan asked for a connecting door on night 1.
    // Alan asked for breakfast on night 1.
    // Alan asked for a parking spot on night 1.
    // Alan asked for an invoice on night 1.
    // Alan asked for a view of the court on night 1.
    // Ada asked for a quiet room on night 2.
    // Ada asked for a late checkout on night 2.
    // Ada asked for a crib on night 2.
    // Ada asked for a high floor on night 2.
    // Ada asked for an extra pillow on night 2.
    // Ada asked for a connecting door on night 2.
    // Ada asked for breakfast on night 2.
    // Ada asked for a parking spot on night 2.
    // Ada asked for an invoice on night 2.
    // Ada asked for a view of the court on night 2.
    // Grace asked for a quiet room on night 2.
    // Grace asked for a late checkout on night 2.
    // Grace asked for a crib on night 2.
    // Grace asked for a high floor on night 2.
    // Grace asked for an extra pillow on night 2.
    // Grace asked for a connecting door on night 2.
    // Grace asked for breakfast on night 2.
    // Grace asked for a parking spot on night 2.
    // Grace asked for an invoice on night 2.
    // Grace asked for a view of the court on night 2.
    // Lin asked for a quiet room on night 2.
    // Lin asked for a late checkout on night 2.
    // Lin asked for a crib on night 2.
    // Lin asked for a high floor on night 2.
    // Lin asked for an extra pillow on night 2.
    // Lin asked for a connecting door on night 2.
    // Lin asked for breakfast on night 2.
    // Lin asked for a parking spot on night 2.
    // Lin asked for an invoice on night 2.
    // Lin asked for a view of the court on night 2.
    // Nikola asked for a quiet room on night 2.
    // Nikola asked for a late checkout on night 2.
    // Nikola asked for a crib on night 2.
    // Nikola asked for a high floor on night 2.
    // Nikola asked for an extra pillow on night 2.
    // Nikola asked for a connecting door on night 2.
    // Nikola asked for breakfast on night 2.
    // Nikola asked for a parking spot on night 2.
    // Nikola asked for an invoice on night 2.
    // Nikola asked for a view of the court on night 2.
    // Joan asked for a quiet room on night 2.
    // Joan asked for a late checkout on night 2.
    // Joan asked for a crib on night 2.
    // Joan asked for a high floor on night 2.
    // Joan asked for an extra pillow on night 2.
    // Joan asked for a connecting door on night 2.
    // Joan asked for breakfast on night 2.
    // Joan asked for a parking spot on night 2.
    // Joan asked for an invoice on night 2.
    // Joan asked for a view of the court on night 2.
    // Alan asked for a quiet room on night 2.
    // Alan asked for a late checkout on night 2.
    // Alan asked for a crib on night 2.
    // Alan asked for a high floor on night 2.
    // Alan asked for an extra pillow on night 2.
    // Alan asked for a connecting door on night 2.
    // Alan asked for breakfast on night 2.
    // Alan asked for a parking spot on night 2.
    // Alan asked for an invoice on night 2.
    // Alan asked for a view of the court on night 2.
    // Ada asked for a quiet room on night 3.
    // Ada asked for a late checkout on night 3.
    // Ada asked for a crib on night 3.
    // Ada asked for a high floor on night 3.
    // Ada asked for an extra pillow on night 3.
    // Ada asked for a connecting door on night 3.
    // Ada asked for breakfast on night 3.
    // Ada asked for a parking spot on night 3.
    // Ada asked for an invoice on night 3.
    // Ada asked for a view of the court on night 3.
    // Grace asked for a quiet room on night 3.
    // Grace asked for a late checkout on night 3.
    // Grace asked for a crib on night 3.
    // Grace asked for a high floor on night 3.
    // Grace asked for an extra pillow on night 3.
    // Grace asked for a connecting door on night 3.
    // Grace asked for breakfast on night 3.
    // Grace asked for a parking spot on night 3.
    // Grace asked for an invoice on night 3.
    // Grace asked for a view of the court on night 3.
    // Lin asked for a quiet room on night 3.
    // Lin asked for a late checkout on night 3.
    // Lin asked for a crib on night 3.
    // Lin asked for a high floor on night 3.
    // Lin asked for an extra pillow on night 3.
    // Lin asked for a connecting door on night 3.
    // Lin asked for breakfast on night 3.
    // Lin asked for a parking spot on night 3.
    // Lin asked for an invoice on night 3.
    // Lin asked for a view of the court on night 3.
    // Nikola asked for a quiet room on night 3.
    // Nikola asked for a late checkout on night 3.
    // Nikola asked for a crib on night 3.
    // Nikola asked for a high floor on night 3.
    // Nikola asked for an extra pillow on night 3.
    // Nikola asked for a connecting door on night 3.
    // Nikola asked for breakfast on night 3.
    // Nikola asked for a parking spot on night 3.
    // Nikola asked for an invoice on night 3.
    // Nikola asked for a view of the court on night 3.
    // Joan asked for a quiet room on night 3.
    // Joan asked for a late checkout on night 3.
    // Joan asked for a crib on night 3.
    // Joan asked for a high floor on night 3.
    // Joan asked for an extra pillow on night 3.
    // Joan asked for a connecting door on night 3.
    // Joan asked for breakfast on night 3.
    // Joan asked for a parking spot on night 3.
    // Joan asked for an invoice on night 3.
    // Joan asked for a view of the court on night 3.
    // Alan asked for a quiet room on night 3.
    // Alan asked for a late checkout on night 3.
    // Alan asked for a crib on night 3.
    // Alan asked for a high floor on night 3.
    // Alan asked for an extra pillow on night 3.
    // Alan asked for a connecting door on night 3.
    // Alan asked for breakfast on night 3.
    // Alan asked for a parking spot on night 3.
    // Alan asked for an invoice on night 3.
    // Alan asked for a view of the court on night 3.
    // Ada asked for a quiet room on night 4.
    // Ada asked for a late checkout on night 4.
    // Ada asked for a crib on night 4.
    // Ada asked for a high floor on night 4.
    // Ada asked for an extra pillow on night 4.
    // Ada asked for a connecting door on night 4.
    // Ada asked for breakfast on night 4.
    // Ada asked for a parking spot on night 4.
    // Ada asked for an invoice on night 4.
    // Ada asked for a view of the court on night 4.
    // Grace asked for a quiet room on night 4.
    // Grace asked for a late checkout on night 4.
    // Grace asked for a crib on night 4.
    // Grace asked for a high floor on night 4.
    // Grace asked for an extra pillow on night 4.
    // Grace asked for a connecting door on night 4.
    // Grace asked for breakfast on night 4.
    // Grace asked for a parking spot on night 4.
    // Grace asked for an invoice on night 4.
    // Grace asked for a view of the court on night 4.
    // Lin asked for a quiet room on night 4.
    // Lin asked for a late checkout on night 4.
    // Lin asked for a crib on night 4.
    // Lin asked for a high floor on night 4.
    // Lin asked for an extra pillow on night 4.
    // Lin asked for a connecting door on night 4.
    // Lin asked for breakfast on night 4.
    // Lin asked for a parking spot on night 4.
    // Lin asked for an invoice on night 4.
    // Lin asked for a view of the court on night 4.
    // Nikola asked for a quiet room on night 4.
    // Nikola asked for a late checkout on night 4.
    // Nikola asked for a crib on night 4.
    // Nikola asked for a high floor on night 4.
    // Nikola asked for an extra pillow on night 4.
    // Nikola asked for a connecting door on night 4.
    // Nikola asked for breakfast on night 4.
    // Nikola asked for a parking spot on night 4.
    // Nikola asked for an invoice on night 4.
    // Nikola asked for a view of the court on night 4.
    // Joan asked for a quiet room on night 4.
    // Joan asked for a late checkout on night 4.
    // Joan asked for a crib on night 4.
    // Joan asked for a high floor on night 4.
    // Joan asked for an extra pillow on night 4.
    // Joan asked for a connecting door on night 4.
    // Joan asked for breakfast on night 4.
    // Joan asked for a parking spot on night 4.
    // Joan asked for an invoice on night 4.
    // Joan asked for a view of the court on night 4.
    // Alan asked for a quiet room on night 4.
    // Alan asked for a late checkout on night 4.
    // Alan asked for a crib on night 4.
    // Alan asked for a high floor on night 4.
    // Alan asked for an extra pillow on night 4.
    // Alan asked for a connecting door on night 4.
    // Alan asked for breakfast on night 4.
    // Alan asked for a parking spot on night 4.
    // Alan asked for an invoice on night 4.
    // Alan asked for a view of the court on night 4.
    bool value = flag;
    bool kept = value;
    return kept;
}

int record() {
    int ledger = 0;
    ledger = 1;
    ledger = 2;
    ledger = 3;
    ledger = 4;
    ledger = 5;
    ledger = 6;
    ledger = 7;
    ledger = 8;
    ledger = 9;
    ledger = 10;
    ledger = 11;
    ledger = 12;
    ledger = 13;
    ledger = 14;
    ledger = 15;
    ledger = 16;
    ledger = 17;
    ledger = 18;
    ledger = 19;
    ledger = 20;
    ledger = 21;
    ledger = 22;
    ledger = 23;
    ledger = 24;
    ledger = 25;
    ledger = 26;
    ledger = 27;
    ledger = 28;
    ledger = 29;
    ledger = 30;
    ledger = 31;
    ledger = 32;
    ledger = 33;
    ledger = 34;
    ledger = 35;
    ledger = 36;
    ledger = 37;
    ledger = 38;
    ledger = 39;
    ledger = 40;
    ledger = 41;
    ledger = 42;
    ledger = 43;
    ledger = 44;
    ledger = 45;
    ledger = 46;
    ledger = 47;
    ledger = 48;
    ledger = 49;
    ledger = 50;
    ledger = 51;
    ledger = 52;
    ledger = 53;
    ledger = 54;
    ledger = 55;
    ledger = 56;
    ledger = 57;
    ledger = 58;
    ledger = 59;
    ledger = 60;
    ledger = 61;
    ledger = 62;
    ledger = 63;
    ledger = 64;
    ledger = 65;
    ledger = 66;
    ledger = 67;
    ledger = 68;
    ledger = 69;
    ledger = 70;
    ledger = 71;
    ledger = 72;
    return ledger;
}
