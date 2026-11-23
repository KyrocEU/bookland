#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>
#include <string.h>

static EKEventStore *nookletEventStore;
static NSString *nookletEventsJSON;

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
    NSData *json = [NSJSONSerialization dataWithJSONObject:@{
        @"title": title, @"grid": visibleGrid
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

void nooklet_calendar_refresh(void) {
    if (nooklet_calendar_status() != 1) return;
    EKEventStore *store = eventStore();
    NSDate *start = [NSDate dateWithTimeIntervalSinceNow:-86400];
    NSDate *end = [NSDate dateWithTimeIntervalSinceNow:86400 * 45];
    NSPredicate *predicate = [store predicateForEventsWithStartDate:start endDate:end calendars:nil];
    NSArray<EKEvent *> *events = [store eventsMatchingPredicate:predicate];
    NSDateFormatter *timedFormat = [NSDateFormatter new];
    timedFormat.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
    timedFormat.dateFormat = @"EEE MMM d '·' HH:mm";
    NSDateFormatter *allDayFormat = [NSDateFormatter new];
    allDayFormat.locale = timedFormat.locale;
    allDayFormat.dateFormat = @"EEE MMM d '·' 'All day'";
    NSMutableArray *rows = [NSMutableArray array];
    for (EKEvent *event in events) {
        if (rows.count >= 80) break;
        [rows addObject:@{
            @"title": event.title ?: @"Untitled event",
            @"start": @([event.startDate timeIntervalSince1970]),
            @"end": @([event.endDate timeIntervalSince1970]),
            @"calendar": event.calendar.title ?: @"Calendar",
            @"all_day": @(event.isAllDay),
            @"display_time": [(event.isAllDay ? allDayFormat : timedFormat)
                stringFromDate:event.startDate] ?: @""
        }];
    }
    NSData *json = [NSJSONSerialization dataWithJSONObject:rows options:0 error:nil];
    nookletEventsJSON = json ? [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding] : @"[]";
}

void nooklet_calendar_connect(void) {
    if (nooklet_calendar_status() == 1) {
        nooklet_calendar_refresh();
        return;
    }
    if (nooklet_calendar_status() == 2) return;
    [eventStore() requestFullAccessToEventsWithCompletion:^(BOOL granted, NSError *error) {
        if (granted && !error) {
            dispatch_async(dispatch_get_main_queue(), ^{ nooklet_calendar_refresh(); });
        }
    }];
}

size_t nooklet_calendar_copy(char *buffer, size_t capacity) {
    if (!buffer || capacity == 0) return 0;
    const char *json = (nookletEventsJSON ?: @"[]").UTF8String;
    size_t len = strlen(json);
    size_t copied = len < capacity - 1 ? len : capacity - 1;
    memcpy(buffer, json, copied);
    buffer[copied] = '\0';
    return copied;
}

int nooklet_calendar_add(const char *title, double unixStart, double durationSeconds) {
    if (nooklet_calendar_status() != 1 || !title || !*title) return 0;
    EKEventStore *store = eventStore();
    EKCalendar *calendar = store.defaultCalendarForNewEvents;
    if (!calendar) return 0;
    EKEvent *event = [EKEvent eventWithEventStore:store];
    event.title = [NSString stringWithUTF8String:title];
    event.startDate = [NSDate dateWithTimeIntervalSince1970:unixStart];
    event.endDate = [event.startDate dateByAddingTimeInterval:MAX(60, durationSeconds)];
    event.calendar = calendar;
    NSError *error = nil;
    BOOL saved = [store saveEvent:event span:EKSpanThisEvent commit:YES error:&error];
    if (saved) nooklet_calendar_refresh();
    return saved ? 1 : 0;
}
