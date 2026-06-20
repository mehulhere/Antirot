package com.mehulhere.antirot;

import android.content.Context;
import android.media.MediaRecorder;
import android.os.Handler;
import android.os.Looper;

import java.io.File;
import java.io.IOException;

public class GentleVoiceRecorder {
    private static final long MINIMUM_CLIP_MS = 10_000;
    private static final long PREFERRED_CLIP_MS = 30_000;
    private static final long HARD_CLIP_MS = 60_000;
    private static final long SETTLED_SILENCE_MS = 1_500;
    private static final int GENTLE_AMPLITUDE = 1_200;

    private final Context context;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private MediaRecorder recorder;
    private File currentFile;
    private long startedAtMs;
    private long lastVoiceAtMs;
    private SegmentCallback callback;
    private boolean recording = false;

    private final Runnable vadTick = new Runnable() {
        @Override
        public void run() {
            evaluateVad();
        }
    };

    public GentleVoiceRecorder(Context context) {
        this.context = context.getApplicationContext();
    }

    public boolean isRecording() {
        return recording;
    }

    public void start(SegmentCallback callback) throws IOException {
        stopTimer();
        currentFile = File.createTempFile("antirot-voice-", ".m4a", context.getCacheDir());
        recorder = new MediaRecorder();
        recorder.setAudioSource(MediaRecorder.AudioSource.MIC);
        recorder.setOutputFormat(MediaRecorder.OutputFormat.MPEG_4);
        recorder.setAudioEncoder(MediaRecorder.AudioEncoder.AAC);
        recorder.setAudioSamplingRate(44_100);
        recorder.setAudioChannels(1);
        recorder.setAudioEncodingBitRate(96_000);
        recorder.setOutputFile(currentFile.getAbsolutePath());
        recorder.prepare();
        recorder.start();
        long now = System.currentTimeMillis();
        startedAtMs = now;
        lastVoiceAtMs = now;
        this.callback = callback;
        recording = true;
        handler.postDelayed(vadTick, 250);
    }

    public File stop() {
        if (!recording) {
            return null;
        }
        stopTimer();
        File file = currentFile;
        try {
            recorder.stop();
        } catch (RuntimeException ignored) {
            if (file != null) {
                file.delete();
            }
            file = null;
        }
        recorder.release();
        recorder = null;
        currentFile = null;
        callback = null;
        recording = false;
        return file;
    }

    private void evaluateVad() {
        if (!recording || recorder == null) {
            return;
        }

        long now = System.currentTimeMillis();
        long elapsed = now - startedAtMs;
        int amplitude = 0;
        try {
            amplitude = recorder.getMaxAmplitude();
        } catch (RuntimeException ignored) {
            amplitude = 0;
        }

        if (amplitude >= GENTLE_AMPLITUDE) {
            lastVoiceAtMs = now;
            handler.postDelayed(vadTick, 250);
            return;
        }

        long silence = now - lastVoiceAtMs;
        boolean settledSilence = silence >= SETTLED_SILENCE_MS;
        boolean shouldFlush = elapsed >= HARD_CLIP_MS ||
                (elapsed >= PREFERRED_CLIP_MS && settledSilence) ||
                (elapsed >= MINIMUM_CLIP_MS && settledSilence);

        if (shouldFlush) {
            SegmentCallback savedCallback = callback;
            File file = stop();
            if (file != null && savedCallback != null) {
                savedCallback.onSegmentReady(file);
            }
            return;
        }

        handler.postDelayed(vadTick, 250);
    }

    private void stopTimer() {
        handler.removeCallbacks(vadTick);
    }

    public interface SegmentCallback {
        void onSegmentReady(File file);
    }
}
