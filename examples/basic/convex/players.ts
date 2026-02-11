import { v } from "convex/values";
import { query, mutation } from "./_generated/server";
import { playerDoc } from "./schema";

/** List all active players (typed array return with nested objects). */
export const listActive = query({
    args: {},
    returns: v.array(playerDoc),
    handler: async (ctx) => {
        return await ctx.db
            .query("players")
            .withIndex("by_isActive", (q) => q.eq("isActive", true))
            .collect();
    },
});

/** Get a single player by ID (typed nullable return). */
export const getById = query({
    args: { playerId: v.id("players") },
    returns: v.union(playerDoc, v.null()),
    handler: async (ctx, { playerId }) => {
        return await ctx.db.get(playerId);
    },
});

/** Get players by rank (typed query with literal union arg). */
export const getByRank = query({
    args: {
        rank: v.union(
            v.literal("bronze"),
            v.literal("silver"),
            v.literal("gold"),
            v.literal("platinum"),
        ),
    },
    returns: v.array(playerDoc),
    handler: async (ctx, { rank }) => {
        return await ctx.db
            .query("players")
            .withIndex("by_rank", (q) => q.eq("rank", rank))
            .collect();
    },
});

/** Create a new player with profile and settings. */
export const create = mutation({
    args: {
        name: v.string(),
        profile: v.object({
            bio: v.optional(v.string()),
            avatar: v.optional(v.string()),
            settings: v.object({
                theme: v.union(v.literal("light"), v.literal("dark")),
                notifications: v.boolean(),
            }),
        }),
    },
    returns: v.id("players"),
    handler: async (ctx, { name, profile }) => {
        return await ctx.db.insert("players", {
            name,
            score: 0,
            isActive: true,
            profile,
            rank: "bronze",
            achievements: [],
            stats: {},
        });
    },
});

/** Update a player's profile using a tagged union action. */
export const updateProfile = mutation({
    args: {
        playerId: v.id("players"),
        action: v.union(
            v.object({ type: v.literal("SetBio"), bio: v.string() }),
            v.object({ type: v.literal("SetAvatar"), avatar: v.string() }),
            v.object({ type: v.literal("UpdateSettings"), theme: v.union(v.literal("light"), v.literal("dark")), notifications: v.boolean() }),
            v.object({ type: v.literal("ClearProfile") }),
        ),
    },
    returns: v.null(),
    handler: async (ctx, { playerId, action }) => {
        const player = await ctx.db.get(playerId);
        if (!player) throw new Error("Player not found");

        switch (action.type) {
            case "SetBio":
                await ctx.db.patch(playerId, {
                    profile: { ...player.profile, bio: action.bio },
                });
                break;
            case "SetAvatar":
                await ctx.db.patch(playerId, {
                    profile: { ...player.profile, avatar: action.avatar },
                });
                break;
            case "UpdateSettings":
                await ctx.db.patch(playerId, {
                    profile: {
                        ...player.profile,
                        settings: {
                            theme: action.theme,
                            notifications: action.notifications,
                        },
                    },
                });
                break;
            case "ClearProfile":
                await ctx.db.patch(playerId, {
                    profile: {
                        settings: player.profile.settings,
                    },
                });
                break;
        }
    },
});

/** Add an achievement to a player. */
export const addAchievement = mutation({
    args: {
        playerId: v.id("players"),
        achievement: v.object({
            name: v.string(),
            unlockedAt: v.number(),
        }),
    },
    returns: v.null(),
    handler: async (ctx, { playerId, achievement }) => {
        const player = await ctx.db.get(playerId);
        if (!player) throw new Error("Player not found");

        await ctx.db.patch(playerId, {
            achievements: [...player.achievements, achievement],
        });
    },
});
