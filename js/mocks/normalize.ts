// Normalizes real Convex validator JSON (kind/fields/element/members) into
// the descriptor format that codegen.rs expects (type/properties/elements/variants).
//
// Real Convex validators serialize to JSON with:
//   { kind: "object", fields: { ... }, isOptional: "required"|"optional", isConvexValidator: true }
//
// Codegen expects:
//   { type: "object", properties: { ... } }
//   { type: "optional", inner: { type: "string" } }

type Descriptor = Record<string, unknown>;

/** Returns true if `val` is a real Convex validator (has isConvexValidator flag). */
function isConvexValidator(val: unknown): val is Descriptor {
	return (
		val !== null &&
		typeof val === "object" &&
		(val as Descriptor).isConvexValidator === true
	);
}

/** Returns true if `val` is already in codegen descriptor format (has `type` string). */
function isCodegenDescriptor(val: unknown): val is Descriptor {
	return (
		val !== null &&
		typeof val === "object" &&
		typeof (val as Descriptor).type === "string" &&
		!(val as Descriptor).isConvexValidator
	);
}

/**
 * Normalize a value that could be either a real Convex validator or a raw
 * record of field→validator. Returns codegen-compatible descriptor.
 */
export function normalize(val: unknown): Descriptor {
	// Already a codegen descriptor (e.g. from mock or paginationOptsValidator)
	if (isCodegenDescriptor(val)) return val as Descriptor;

	// Real Convex validator object
	if (isConvexValidator(val)) return normalizeValidator(val);

	// Raw record of { fieldName: validator } — wrap as object
	if (val !== null && typeof val === "object" && !Array.isArray(val)) {
		const properties: Record<string, Descriptor> = {};
		for (const [key, v] of Object.entries(val as Record<string, unknown>)) {
			properties[key] = normalize(v);
		}
		return { type: "object", properties };
	}

	// Fallback — treat as opaque
	return { type: "any" };
}

function normalizeValidator(v: Descriptor): Descriptor {
	const kind = v.kind as string;
	const isOpt = v.isOptional === "optional";

	// Build the inner descriptor based on kind
	let inner: Descriptor;

	switch (kind) {
		// Primitives — map directly
		case "string":
		case "boolean":
		case "int64":
		case "null":
		case "any":
		case "bytes":
			inner = { type: kind };
			break;

		// Real Convex v.number() → kind:"float64", but codegen.rs expects type:"number"
		case "float64":
			inner = { type: "number" };
			break;

		case "id":
			inner = { type: "id", tableName: v.tableName as string };
			break;

		case "literal":
			inner = { type: "literal", value: v.value };
			break;

		case "object": {
			const fields = v.fields as Record<string, unknown> | undefined;
			const properties: Record<string, Descriptor> = {};
			if (fields) {
				for (const [key, fieldVal] of Object.entries(fields)) {
					properties[key] = normalize(fieldVal);
				}
			}
			inner = { type: "object", properties };
			break;
		}

		case "array": {
			const element = v.element as unknown;
			inner = { type: "array", elements: normalize(element) };
			break;
		}

		case "union": {
			const members = v.members as unknown[];
			inner = {
				type: "union",
				variants: members.map((m) => normalize(m)),
			};
			break;
		}

		case "record": {
			inner = {
				type: "record",
				keyType: normalize(v.key),
				valueType: normalize(v.value),
			};
			break;
		}

		default:
			// Unknown kind — treat as any
			inner = { type: "any" };
			break;
	}

	// Real Convex merges optional into the inner validator via isOptional flag.
	// Codegen expects { type: "optional", inner: ... } wrapper.
	if (isOpt) {
		return { type: "optional", inner };
	}

	return inner;
}
