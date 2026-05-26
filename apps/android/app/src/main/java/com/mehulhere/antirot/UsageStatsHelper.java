package com.mehulhere.antirot;

import android.app.AppOpsManager;
import android.app.usage.UsageStats;
import android.app.usage.UsageStatsManager;
import android.content.Context;
import android.content.Intent;
import android.os.Process;
import android.provider.Settings;

import java.util.List;

public class UsageStatsHelper {
    private final Context context;

    public UsageStatsHelper(Context context) {
        this.context = context.getApplicationContext();
    }

    public boolean hasUsageAccess() {
        AppOpsManager appOps = context.getSystemService(AppOpsManager.class);
        if (appOps == null) {
            return false;
        }
        int mode = appOps.checkOpNoThrow(
                AppOpsManager.OPSTR_GET_USAGE_STATS,
                Process.myUid(),
                context.getPackageName()
        );
        return mode == AppOpsManager.MODE_ALLOWED;
    }

    public void openUsageAccessSettings() {
        Intent intent = new Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        context.startActivity(intent);
    }

    public UsageSummary lastThirtyMinutes() {
        if (!hasUsageAccess()) {
            return new UsageSummary(0, 0, false);
        }
        UsageStatsManager manager = context.getSystemService(UsageStatsManager.class);
        if (manager == null) {
            return new UsageSummary(0, 0, true);
        }
        long end = System.currentTimeMillis();
        long start = end - 30 * 60_000L;
        List<UsageStats> stats = manager.queryUsageStats(UsageStatsManager.INTERVAL_DAILY, start, end);
        long total = 0;
        int count = 0;
        for (UsageStats item : stats) {
            long foreground = item.getTotalTimeInForeground();
            if (foreground > 0) {
                total += Math.min(foreground, end - start);
                count++;
            }
        }
        return new UsageSummary(total, count, true);
    }
}
