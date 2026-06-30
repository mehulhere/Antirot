package com.mehulhere.antirot;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class CoachQuickAction {
    public final String id;
    public final String title;
    public final String message;
    public final boolean fillsDraft;

    private CoachQuickAction(String id, String title, String message, boolean fillsDraft) {
        this.id = id;
        this.title = title;
        this.message = message;
        this.fillsDraft = fillsDraft;
    }

    public static List<CoachQuickAction> forState(String runtimeState) {
        Map<String, CoachQuickAction> byId = allById();
        String state = runtimeState == null ? "unknown" : runtimeState.toLowerCase();
        String[] ids;
        switch (state) {
            case "onboarding":
                ids = new String[] {};
                break;
            case "idle":
                ids = new String[] {"start_working"};
                break;
            case "working":
                ids = new String[] {"done"};
                break;
            default:
                ids = new String[] {};
                break;
        }

        List<CoachQuickAction> actions = new ArrayList<>();
        for (String id : ids) {
            CoachQuickAction action = byId.get(id);
            if (action != null) {
                actions.add(action);
            }
        }
        return actions;
    }

    private static Map<String, CoachQuickAction> allById() {
        Map<String, CoachQuickAction> actions = new HashMap<>();
        add(actions, new CoachQuickAction(
                "start_working",
                "Ready Work",
                "I am ready to work. Start the next serious work block.",
                false
        ));
        add(actions, new CoachQuickAction(
                "done",
                "Done",
                "Done. I finished the current work block. Log it and tell me the next move.",
                false
        ));
        add(actions, new CoachQuickAction(
                "need_break",
                "Real Break",
                "I need a real break. Help me choose the minimum honest break.",
                false
        ));
        add(actions, new CoachQuickAction(
                "log_work",
                "Log Work",
                "Log work: I worked on ",
                true
        ));
        add(actions, new CoachQuickAction(
                "wake_up",
                "Awake",
                "I am awake. Log it and tell me the first concrete move.",
                false
        ));
        add(actions, new CoachQuickAction(
                "movie_break",
                "Movie Check",
                "I want a 2 hour movie break because I deserve it. Please please.",
                false
        ));
        return actions;
    }

    private static void add(Map<String, CoachQuickAction> actions, CoachQuickAction action) {
        actions.put(action.id, action);
    }
}
