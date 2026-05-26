package com.mehulhere.antirot;

public class UsageSummary {
    public final long totalForegroundMillis;
    public final int appCount;
    public final boolean permissionGranted;

    public UsageSummary(long totalForegroundMillis, int appCount, boolean permissionGranted) {
        this.totalForegroundMillis = totalForegroundMillis;
        this.appCount = appCount;
        this.permissionGranted = permissionGranted;
    }

    public String format() {
        if (!permissionGranted) {
            return "Usage access is not granted.";
        }
        long minutes = Math.round(totalForegroundMillis / 60_000.0);
        return "Last 30 min: about " + minutes + " min foreground usage across " + appCount + " app(s).";
    }
}
