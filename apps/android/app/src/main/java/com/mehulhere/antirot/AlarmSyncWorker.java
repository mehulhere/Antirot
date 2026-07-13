package com.mehulhere.antirot;

import android.content.Context;

import androidx.annotation.NonNull;
import androidx.work.Constraints;
import androidx.work.ExistingPeriodicWorkPolicy;
import androidx.work.ExistingWorkPolicy;
import androidx.work.NetworkType;
import androidx.work.OneTimeWorkRequest;
import androidx.work.PeriodicWorkRequest;
import androidx.work.WorkManager;
import androidx.work.Worker;
import androidx.work.WorkerParameters;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

public class AlarmSyncWorker extends Worker {
    private static final String PERIODIC_WORK = "antirot-alarm-sync";
    private static final String IMMEDIATE_WORK = "antirot-alarm-sync-now";

    public AlarmSyncWorker(@NonNull Context context, @NonNull WorkerParameters parameters) {
        super(context, parameters);
    }

    public static void schedule(Context context) {
        Constraints constraints = new Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build();
        PeriodicWorkRequest request = new PeriodicWorkRequest.Builder(
                AlarmSyncWorker.class,
                15,
                TimeUnit.MINUTES
        ).setConstraints(constraints).build();
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                PERIODIC_WORK,
                ExistingPeriodicWorkPolicy.UPDATE,
                request
        );
    }

    public static void runNow(Context context) {
        OneTimeWorkRequest request = new OneTimeWorkRequest.Builder(AlarmSyncWorker.class).build();
        WorkManager.getInstance(context).enqueueUniqueWork(
                IMMEDIATE_WORK,
                ExistingWorkPolicy.REPLACE,
                request
        );
    }

    @NonNull
    @Override
    public Result doWork() {
        SettingsStore settings = new SettingsStore(getApplicationContext());
        if (settings.getApiToken().isEmpty()) {
            return Result.success();
        }
        AntirotApiClient api = new AntirotApiClient(getApplicationContext());
        CountDownLatch completed = new CountDownLatch(1);
        AtomicBoolean success = new AtomicBoolean(false);
        api.fetchPendingAlarms(new AntirotApiClient.AlarmCallback() {
            @Override
            public void onAlarms(
                    List<AlarmJob> alarms,
                    List<String> cancelledSeriesIds,
                    List<String> cancelledAlarmIds
            ) {
                AlarmScheduler scheduler = new AlarmScheduler(getApplicationContext());
                scheduler.cancelSeries(cancelledSeriesIds);
                for (String alarmId : cancelledAlarmIds) {
                    scheduler.cancelAlarm(alarmId);
                }
                List<AlarmJob> scheduled = new ArrayList<>();
                for (AlarmJob alarm : alarms) {
                    if (scheduler.schedule(alarm).scheduled) {
                        scheduled.add(alarm);
                    }
                }
                api.reconcileAlarms(scheduled, cancelledSeriesIds, message -> {
                    success.set(!message.startsWith("🔴 FALLBACK:"));
                    completed.countDown();
                });
            }

            @Override
            public void onResult(String message) {
                completed.countDown();
            }
        });
        try {
            if (!completed.await(45, TimeUnit.SECONDS)) {
                return Result.retry();
            }
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            return Result.retry();
        }
        return success.get() ? Result.success() : Result.retry();
    }
}
