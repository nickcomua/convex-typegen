import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// =============================================================================
// Shared validators (for typed returns)
// =============================================================================

export const gameStatus = v.union(
  v.literal("active"),
  v.literal("finished"),
  v.literal("abandoned"),
);

export const gameDoc = v.object({
  _id: v.id("games"),
  _creationTime: v.number(),
  win_count: v.number(),
  loss_count: v.number(),
  status: gameStatus,
  lastPlayedAt: v.optional(v.number()),
});

export const playerDoc = v.object({
  _id: v.id("players"),
  _creationTime: v.number(),
  name: v.string(),
  score: v.number(),
  isActive: v.boolean(),
  profile: v.object({
    bio: v.optional(v.string()),
    avatar: v.optional(v.string()),
    settings: v.object({
      theme: v.union(v.literal("light"), v.literal("dark")),
      notifications: v.boolean(),
    }),
  }),
  rank: v.union(
    v.literal("bronze"),
    v.literal("silver"),
    v.literal("gold"),
    v.literal("platinum"),
  ),
  achievements: v.array(v.object({
    name: v.string(),
    unlockedAt: v.number(),
  })),
  stats: v.record(v.string(), v.number()),
});

// Comprehensive test table exercising all convex data types and variants
export const testDoc = v.object({
  _id: v.id("test"),
  _creationTime: v.number(),
  testId: v.id("test"),
  nullField: v.null(),
  bigNum: v.int64(),
  score: v.number(),
  isActive: v.boolean(),
  label: v.string(),
  rawData: v.bytes(),
  simpleTags: v.array(v.string()),
  numberArray: v.array(v.number()),
  nestedArray: v.array(v.array(v.string())),
  simpleObject: v.object({ key: v.string() }),
  complexObject: v.object({
    name: v.string(),
    count: v.number(),
    flag: v.boolean(),
    optional: v.optional(v.string()),
  }),
  nestedObject: v.object({
    level1: v.object({
      level2: v.object({
        value: v.string(),
      }),
    }),
  }),
  stringRecord: v.record(v.string(), v.string()),
  numberRecord: v.record(v.string(), v.number()),
  mixedUnion: v.union(v.string(), v.number()),
  literalUnion: v.union(
    v.literal("draft"),
    v.literal("published"),
    v.literal("archived"),
  ),
  taggedUnion: v.union(
    v.object({ type: v.literal("click"), x: v.number(), y: v.number() }),
    v.object({ type: v.literal("scroll"), delta: v.number() }),
    v.object({ type: v.literal("keypress") }),
  ),
  optionalString: v.optional(v.string()),
  optionalObject: v.optional(v.object({
    theme: v.string(),
  })),
  optionalArray: v.optional(v.array(v.string())),
  nullable: v.union(v.string(), v.null()),
  complexNested: v.object({
    metadata: v.object({
      tags: v.array(v.string()),
      counts: v.record(v.string(), v.number()),
      status: v.union(v.literal("active"), v.literal("inactive")),
    }),
    settings: v.optional(v.object({
      preferences: v.record(v.string(), v.string()),
      limits: v.array(v.number()),
    })),
  }),
});

// https://docs.convex.dev/database/types
export default defineSchema({
  games: defineTable({
    win_count: v.number(),
    loss_count: v.number(),
    status: gameStatus,
    lastPlayedAt: v.optional(v.number()),
  }),

  players: defineTable({
    name: v.string(),
    score: v.number(),
    isActive: v.boolean(),
    profile: v.object({
      bio: v.optional(v.string()),
      avatar: v.optional(v.string()),
      settings: v.object({
        theme: v.union(v.literal("light"), v.literal("dark")),
        notifications: v.boolean(),
      }),
    }),
    rank: v.union(
      v.literal("bronze"),
      v.literal("silver"),
      v.literal("gold"),
      v.literal("platinum"),
    ),
    achievements: v.array(v.object({
      name: v.string(),
      unlockedAt: v.number(),
    })),
    stats: v.record(v.string(), v.number()),
  })
    .index("by_rank", ["rank"])
    .index("by_isActive", ["isActive"]),

  // Comprehensive test table exercising all convex data types and variants
  test: defineTable({
    // Basic Types
    testId: v.id("test"),
    nullField: v.null(),
    bigNum: v.int64(),
    score: v.number(),
    isActive: v.boolean(),
    label: v.string(),
    rawData: v.bytes(),

    // Arrays
    simpleTags: v.array(v.string()),
    numberArray: v.array(v.number()),
    nestedArray: v.array(v.array(v.string())),

    // Objects
    simpleObject: v.object({
      key: v.string(),
    }),
    complexObject: v.object({
      name: v.string(),
      count: v.number(),
      flag: v.boolean(),
      optional: v.optional(v.string()),
    }),
    nestedObject: v.object({
      level1: v.object({
        level2: v.object({
          value: v.string(),
        }),
      }),
    }),

    // Records
    stringRecord: v.record(v.string(), v.string()),
    numberRecord: v.record(v.string(), v.number()),

    // Unions
    mixedUnion: v.union(v.string(), v.number()),
    literalUnion: v.union(
      v.literal("draft"),
      v.literal("published"),
      v.literal("archived"),
    ),
    taggedUnion: v.union(
      v.object({ type: v.literal("click"), x: v.number(), y: v.number() }),
      v.object({ type: v.literal("scroll"), delta: v.number() }),
      v.object({ type: v.literal("keypress") }),
    ),

    // Optionals
    optionalString: v.optional(v.string()),
    optionalObject: v.optional(v.object({
      theme: v.string(),
    })),
    optionalArray: v.optional(v.array(v.string())),

    // Nullable (union with null)
    nullable: v.union(v.string(), v.null()),

    // Complex nested
    complexNested: v.object({
      metadata: v.object({
        tags: v.array(v.string()),
        counts: v.record(v.string(), v.number()),
        status: v.union(v.literal("active"), v.literal("inactive")),
      }),
      settings: v.optional(v.object({
        preferences: v.record(v.string(), v.string()),
        limits: v.array(v.number()),
      })),
    }),
  }),
});
