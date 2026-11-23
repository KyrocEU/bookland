#import <AppKit/AppKit.h>
#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>
#include <string.h>

static EKEventStore *nookletEventStore;
static BOOL nookletCalendarPermissionPending;
static NSString *nookletEventsJSON;
static unsigned long long nookletEventsGeneration;
static unsigned long long nookletEventsVersion;
static EKEventStore *eventStore(void);
int nooklet_calendar_status(void);

static dispatch_queue_t calendarQueryQueue(void) {
    static dispatch_queue_t queue;
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        queue = dispatch_queue_create("com.bookland.calendar-query", DISPATCH_QUEUE_SERIAL);
    });
    return queue;
}

size_t nooklet_calendar_calendars_copy(char *buffer, size_t capacity) {
    if (!buffer || capacity == 0) return 0;
    if (nooklet_calendar_status() != 1) {
        buffer[0] = '\0';
        return 0;
    }
    NSMutableArray *rows = [NSMutableArray array];
    for (EKCalendar *calendar in [eventStore() calendarsForEntityType:EKEntityTypeEvent]) {
        if (rows.count >= 100) break;
        [rows addObject:@{
            @"id": calendar.calendarIdentifier ?: @"",
            @"title": calendar.title ?: @"Calendar",
            @"source": calendar.source.title ?: @"Mac Calendar",
            @"writable": @(calendar.allowsContentModifications)
        }];
    }
    NSData *json = [NSJSONSerialization dataWithJSONObject:rows options:0 error:nil];
    if (!json || json.length >= capacity) { buffer[0] = '\0'; return 0; }
    memcpy(buffer, json.bytes, json.length);
    buffer[json.length] = '\0';
    return json.length;
}

static NSCalendar *booklandCalendar(void) {
    NSCalendar *calendar = [NSCalendar calendarWithIdentifier:NSCalendarIdentifierGregorian];
    calendar.timeZone = NSTimeZone.localTimeZone;
    return calendar;
}

int nooklet_calendar_month_index(void) {
    NSDateComponents *date = [booklandCalendar() components:
        NSCalendarUnitYear | NSCalendarUnitMonth fromDate:[NSDate date]];
    return (int)(date.year * 12 + date.month);
}

size_t nooklet_calendar_month(int offset, char *buffer, size_t capacity) {
    if (!buffer || capacity == 0) return 0;
    NSCalendar *calendar = booklandCalendar();
    NSDateComponents *current = [calendar components:NSCalendarUnitYear | NSCalendarUnitMonth
        fromDate:[NSDate date]];
    current.day = 1;
    NSDateComponents *shift = [NSDateComponents new];
    shift.month = offset;
    NSDate *first = [calendar dateByAddingComponents:shift
        toDate:[calendar dateFromComponents:current] options:0];
    if (!first) { buffer[0] = '\0'; return 0; }
    NSDateFormatter *formatter = [NSDateFormatter new];
    formatter.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
    formatter.timeZone = calendar.timeZone;
    formatter.dateFormat = @"MMMM yyyy";
    NSString *title = [formatter stringFromDate:first] ?: @"Calendar";
    NSInteger weekday = [calendar component:NSCalendarUnitWeekday fromDate:first] - 1;
    NSInteger days = [calendar rangeOfUnit:NSCalendarUnitDay
        inUnit:NSCalendarUnitMonth forDate:first].length;
    NSMutableString *grid = [NSMutableString string];
    NSInteger left = MAX(0, (22 - (NSInteger)title.length) / 2 - 1);
    for (NSInteger index = 0; index < left; index++) [grid appendString:@" "];
    [grid appendString:title];
    for (NSInteger index = left + title.length; index < 22; index++) [grid appendString:@" "];
    [grid appendString:@"\nSu Mo Tu We Th Fr Sa  \n"];
    for (NSInteger row = 0; row < 6; row++) {
        for (NSInteger column = 0; column < 7; column++) {
            NSInteger day = row * 7 + column - weekday + 1;
            if (day >= 1 && day <= days) {
                [grid appendFormat:@"%2ld ", (long)day];
            } else {
                [grid appendString:@"   "];
            }
        }
        [grid appendString:@" \n"];
    }
    // The previous Rust run_output() trimmed `cal` before displaying it.
    // Keep that visible height and first-line alignment exactly the same.
    NSString *visibleGrid = [grid stringByTrimmingCharactersInSet:
        NSCharacterSet.whitespaceAndNewlineCharacterSet];
    BOOL isCurrentMonth = current.year == [calendar component:NSCalendarUnitYear fromDate:first]
        && current.month == [calendar component:NSCalendarUnitMonth fromDate:first];
    NSInteger today = isCurrentMonth
        ? [calendar component:NSCalendarUnitDay fromDate:[NSDate date]] : 0;
    NSData *json = [NSJSONSerialization dataWithJSONObject:@{
        @"title": title, @"grid": visibleGrid,
        @"year": @([calendar component:NSCalendarUnitYear fromDate:first]),
        @"month": @([calendar component:NSCalendarUnitMonth fromDate:first]),
        @"first_weekday": @(weekday), @"days_in_month": @(days),
        @"today": @(today)
    } options:0 error:nil];
    if (!json) { buffer[0] = '\0'; return 0; }
    size_t copied = MIN(json.length, capacity - 1);
    memcpy(buffer, json.bytes, copied);
    buffer[copied] = '\0';
    return copied;
}

static EKEventStore *eventStore(void) {
    if (!nookletEventStore) nookletEventStore = [EKEventStore new];
    return nookletEventStore;
}

int nooklet_calendar_status(void) {
    EKAuthorizationStatus status = [EKEventStore authorizationStatusForEntityType:EKEntityTypeEvent];
    if (status == EKAuthorizationStatusFullAccess) return 1;
    if (status == EKAuthorizationStatusDenied || status == EKAuthorizationStatusRestricted) return 2;
    return 0;
}

void nooklet_calendar_refresh(const char *calendarID, int monthOffset) {
    if (nooklet_calendar_status() != 1) return;
    NSString *identifier = calendarID && *calendarID
        ? [NSString stringWithUTF8String:calendarID] : nil;
    unsigned long long generation;
    @synchronized([EKEventStore class]) {
        generation = ++nookletEventsGeneration;
    }
    dispatch_async(calendarQueryQueue(), ^{
        @autoreleasepool {
            EKEventStore *store = [EKEventStore new];
            NSArray<EKCalendar *> *calendars = nil;
            if (identifier) {
                EKCalendar *selected = [store calendarWithIdentifier:identifier];
                calendars = selected ? @[selected] : @[];
            }
            NSMutableArray<EKEvent *> *events = [NSMutableArray array];
            if (!identifier || calendars.count != 0) {
                NSCalendar *calendar = booklandCalendar();
                NSDate *now = [NSDate date];
                NSDate *upcomingStart = [now dateByAddingTimeInterval:-86400];
                NSDate *upcomingEnd = [now dateByAddingTimeInterval:86400 * 45];
                NSDateComponents *shift = [NSDateComponents new];
                shift.month = monthOffset;
                NSDate *shifted = [calendar dateByAddingComponents:shift toDate:now options:0];
                NSDateComponents *parts = [calendar components:
                    NSCalendarUnitYear | NSCalendarUnitMonth fromDate:shifted];
                parts.day = 1;
                NSDate *monthStart = [calendar dateFromComponents:parts];
                NSDate *monthEnd = [calendar dateByAddingUnit:NSCalendarUnitMonth value:1
                    toDate:monthStart options:0];
                BOOL overlaps = [monthStart compare:upcomingEnd] != NSOrderedDescending
                    && [monthEnd compare:upcomingStart] != NSOrderedAscending;
                NSArray<NSArray<NSDate *> *> *ranges = overlaps
                    ? @[@[[monthStart earlierDate:upcomingStart],
                          [monthEnd laterDate:upcomingEnd]]]
                    : @[@[upcomingStart, upcomingEnd], @[monthStart, monthEnd]];
                for (NSArray<NSDate *> *range in ranges) {
                    NSPredicate *predicate = [store predicateForEventsWithStartDate:range[0]
                        endDate:range[1] calendars:calendars];
                    NSArray<EKEvent *> *matching = [store eventsMatchingPredicate:predicate];
                    if (matching) [events addObjectsFromArray:matching];
                }
                [events sortUsingComparator:^NSComparisonResult(EKEvent *a, EKEvent *b) {
                    return [a.startDate compare:b.startDate];
                }];
            }
            NSDateFormatter *timedFormat = [NSDateFormatter new];
            timedFormat.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
            timedFormat.dateFormat = @"EEE MMM d '·' HH:mm";
            NSDateFormatter *allDayFormat = [NSDateFormatter new];
            allDayFormat.locale = timedFormat.locale;
            allDayFormat.dateFormat = @"EEE MMM d '·' 'All day'";
            NSDateFormatter *dayKeyFormat = [NSDateFormatter new];
            dayKeyFormat.locale = timedFormat.locale;
            dayKeyFormat.timeZone = NSTimeZone.localTimeZone;
            dayKeyFormat.dateFormat = @"yyyy-MM-dd";
            NSMutableArray *rows = [NSMutableArray array];
            for (EKEvent *event in events) {
                if (rows.count >= 160) break;
                [rows addObject:@{
                    @"title": event.title ?: @"Untitled event",
                    @"start": @([event.startDate timeIntervalSince1970]),
                    @"end": @([event.endDate timeIntervalSince1970]),
                    @"calendar": event.calendar.title ?: @"Calendar",
                    @"day_key": [dayKeyFormat stringFromDate:event.startDate] ?: @"",
                    @"all_day": @(event.isAllDay),
                    @"display_time": [(event.isAllDay ? allDayFormat : timedFormat)
                        stringFromDate:event.startDate] ?: @""
                }];
            }
            NSData *json = [NSJSONSerialization dataWithJSONObject:rows options:0 error:nil];
            NSString *result = json
                ? [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding] : @"[]";
            @synchronized([EKEventStore class]) {
                if (generation == nookletEventsGeneration) {
                    nookletEventsJSON = result;
                    ++nookletEventsVersion;
                }
            }
        }
    });
}

int nooklet_calendar_request_pending(void) {
    @synchronized([EKEventStore class]) {
        return nookletCalendarPermissionPending ? 1 : 0;
    }
}

void nooklet_calendar_connect(void) {
    if (nooklet_calendar_status() != 0) return;
    @synchronized([EKEventStore class]) {
        if (nookletCalendarPermissionPending) return;
        nookletCalendarPermissionPending = YES;
    }
    // The island can receive a click while another app is still active.
    // Make this explicit user request eligible for a macOS privacy sheet.
    if (NSApp && !NSApp.isActive) [NSApp activate];
    [eventStore() requestFullAccessToEventsWithCompletion:^(BOOL granted, NSError *error) {
        @synchronized([EKEventStore class]) {
            nookletCalendarPermissionPending = NO;
        }
        if (!granted && error) {
            NSLog(@"Bookland Calendar access request failed: %@", error.localizedDescription);
        }
    }];
}

unsigned long long nooklet_calendar_snapshot_version(void) {
    @synchronized([EKEventStore class]) { return nookletEventsVersion; }
}

size_t nooklet_calendar_copy(char *buffer, size_t capacity) {
    if (!buffer || capacity == 0) return 0;
    @synchronized([EKEventStore class]) {
        const char *json = (nookletEventsJSON ?: @"[]").UTF8String;
        size_t len = strlen(json);
        if (len >= capacity) { buffer[0] = '\0'; return 0; }
        memcpy(buffer, json, len);
        buffer[len] = '\0';
        return len;
    }
}

int nooklet_calendar_add(const char *title, double unixStart, double durationSeconds,
                         const char *calendarID) {
    if (nooklet_calendar_status() != 1 || !title || !*title
        || !calendarID || !*calendarID) return 0;
    EKEventStore *store = eventStore();
    EKCalendar *calendar = [store calendarWithIdentifier:
        [NSString stringWithUTF8String:calendarID]];
    if (!calendar || !calendar.allowsContentModifications) return 0;
    EKEvent *event = [EKEvent eventWithEventStore:store];
    event.title = [NSString stringWithUTF8String:title];
    event.startDate = [NSDate dateWithTimeIntervalSince1970:unixStart];
    event.endDate = [event.startDate dateByAddingTimeInterval:MAX(60, durationSeconds)];
    event.calendar = calendar;
    NSError *error = nil;
    BOOL saved = [store saveEvent:event span:EKSpanThisEvent commit:YES error:&error];
    if (saved) nooklet_calendar_refresh(calendarID, 0);
    return saved ? 1 : 0;
}
