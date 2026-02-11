import { v } from "convex/values";
import { query, mutation, QueryCtx } from "./_generated/server";
import { gameDoc, gameStatus } from "./schema";

/** Get the current game record (typed return). */
export const getGame = query({
    args: {},
    returns: v.union(gameDoc, v.null()),
    handler: async (ctx) => {
        return await getGameData(ctx);
    },
});

/** List all games (typed array return). */
export const listGames = query({
    args: {},
    returns: v.array(gameDoc),
    handler: async (ctx) => {
        return await ctx.db.query("games").collect();
    },
});

/** Get games filtered by status. */
export const getByStatus = query({
    args: { status: gameStatus },
    returns: v.array(gameDoc),
    handler: async (ctx, { status }) => {
        return (await ctx.db.query("games").collect())
            .filter((g) => g.status === status);
    },
});

/** Record a win. */
export const winGame = mutation({
    args: {},
    returns: v.null(),
    handler: async (ctx) => {
        let game = await getGameData(ctx);

        if (!game) {
            await ctx.db.insert("games", {
                win_count: 1,
                loss_count: 0,
                status: "active",
                lastPlayedAt: Date.now(),
            });
        } else {
            await ctx.db.patch(game._id, {
                win_count: game.win_count + 1,
                lastPlayedAt: Date.now(),
            });
        }
    },
});

/** Record a loss. */
export const lossGame = mutation({
    args: {},
    returns: v.null(),
    handler: async (ctx) => {
        let game = await getGameData(ctx);

        if (!game) {
            await ctx.db.insert("games", {
                win_count: 0,
                loss_count: 1,
                status: "active",
                lastPlayedAt: Date.now(),
            });
        } else {
            await ctx.db.patch(game._id, {
                loss_count: game.loss_count + 1,
                lastPlayedAt: Date.now(),
            });
        }
    },
});

/** Update game status using a tagged union result. */
export const updateGameStatus = mutation({
    args: {
        gameId: v.id("games"),
        result: v.union(
            v.object({ type: v.literal("Win"), bonus: v.number() }),
            v.object({ type: v.literal("Loss"), penalty: v.number() }),
            v.object({ type: v.literal("Draw") }),
        ),
    },
    returns: v.null(),
    handler: async (ctx, { gameId, result }) => {
        const game = await ctx.db.get(gameId);
        if (!game) throw new Error("Game not found");

        const now = Date.now();

        switch (result.type) {
            case "Win":
                await ctx.db.patch(gameId, {
                    win_count: game.win_count + 1 + result.bonus,
                    lastPlayedAt: now,
                });
                break;
            case "Loss":
                await ctx.db.patch(gameId, {
                    loss_count: game.loss_count + 1 + result.penalty,
                    lastPlayedAt: now,
                });
                break;
            case "Draw":
                await ctx.db.patch(gameId, {
                    lastPlayedAt: now,
                });
                break;
        }
    },
});

async function getGameData(ctx: QueryCtx)
{
    return await ctx.db.query("games").first();
}
