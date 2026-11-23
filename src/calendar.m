#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>
#include <string.h>

static EKEventStore *nookletEventStore;
static NSString *nookletEventsJSON;

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
    NSMutableArray *rows = [NSMutableArray array];
    for (EKEvent *event in events) {
        if (rows.count >= 80) break;
        [rows addObject:@{
            @"title": event.title ?: @"Untitled event",
            @"start": @([event.startDate timeIntervalSince1970]),
            @"end": @([event.endDate timeIntervalSince1970]),
            @"calendar": event.calendar.title ?: @"Calendar",
            @"all_day": @(event.isAllDay)
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
