import XCTest
@testable import Antirot

final class SoundLibraryTests: XCTestCase {
    func testNotificationSoundAcceptsOnlyAppleContainerFormats() {
        XCTAssertTrue(SoundLibrary.isSupportedNotificationSoundExtension("aiff"))
        XCTAssertTrue(SoundLibrary.isSupportedNotificationSoundExtension("wav"))
        XCTAssertTrue(SoundLibrary.isSupportedNotificationSoundExtension("caf"))
        XCTAssertFalse(SoundLibrary.isSupportedNotificationSoundExtension("m4a"))
        XCTAssertFalse(SoundLibrary.isSupportedNotificationSoundExtension("mp3"))
    }

    func testNotificationSoundMustBeStrictlyUnderThirtySeconds() {
        XCTAssertTrue(SoundLibrary.isValidNotificationSoundDuration(29.999))
        XCTAssertFalse(SoundLibrary.isValidNotificationSoundDuration(0))
        XCTAssertFalse(SoundLibrary.isValidNotificationSoundDuration(30))
        XCTAssertFalse(SoundLibrary.isValidNotificationSoundDuration(.infinity))
    }
}
