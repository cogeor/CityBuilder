import { describe, it, expect, beforeEach } from "vitest";
import {
  RenderPass,
  INSTANCE_BYTE_SIZE,
  DEFAULT_INSTANCE,
  packInstance,
  unpackInstance,
  RenderInstanceBuilder,
  type RenderInstance,
} from "../types.js";

describe("types", () => {
  // ─── RenderPass ───────────────────────────────────────────────────────────

  it("RenderPass enum values are 0 through 6", () => {
    expect(RenderPass.Terrain).toBe(0);
    expect(RenderPass.Networks).toBe(1);
    expect(RenderPass.Buildings).toBe(2);
    expect(RenderPass.Props).toBe(3);
    expect(RenderPass.Automata).toBe(4);
    expect(RenderPass.Overlays).toBe(5);
    expect(RenderPass.UI).toBe(6);
  });

  // ─── INSTANCE_BYTE_SIZE ───────────────────────────────────────────────────

  it("INSTANCE_BYTE_SIZE is 48", () => {
    expect(INSTANCE_BYTE_SIZE).toBe(48);
  });

  // ─── DEFAULT_INSTANCE ─────────────────────────────────────────────────────

  it("DEFAULT_INSTANCE has correct values", () => {
    expect(DEFAULT_INSTANCE.sprite_id).toBe(0);
    expect(DEFAULT_INSTANCE.atlas_id).toBe(0);
    expect(DEFAULT_INSTANCE.screen_x).toBe(0);
    expect(DEFAULT_INSTANCE.screen_y).toBe(0);
    expect(DEFAULT_INSTANCE.z_order).toBe(0);
    expect(DEFAULT_INSTANCE.palette_id).toBe(0);
    expect(DEFAULT_INSTANCE.mask_flags).toBe(0);
    expect(DEFAULT_INSTANCE.anim_frame).toBe(0);
    expect(DEFAULT_INSTANCE.render_flags).toBe(1); // visible
    expect(DEFAULT_INSTANCE.scale_x).toBe(1.0);
    expect(DEFAULT_INSTANCE.scale_y).toBe(1.0);
    expect(DEFAULT_INSTANCE.rotation).toBe(0);
    expect(DEFAULT_INSTANCE.tint_r).toBe(255);
    expect(DEFAULT_INSTANCE.tint_g).toBe(255);
    expect(DEFAULT_INSTANCE.tint_b).toBe(255);
    expect(DEFAULT_INSTANCE.tint_a).toBe(255);
  });

  // ─── Pack / Unpack ────────────────────────────────────────────────────────

  it("pack/unpack round-trip preserves all fields", () => {
    const instance: RenderInstance = {
      sprite_id: 42,
      atlas_id: 3,
      screen_x: 512.5,
      screen_y: 768.25,
      z_order: 1000,
      palette_id: 7,
      mask_flags: 0x03,
      anim_frame: 12,
      render_flags: 1,
      scale_x: 2.0,
      scale_y: 0.5,
      rotation: 1.5707963,
      tint_r: 200,
      tint_g: 100,
      tint_b: 50,
      tint_a: 128,
    };

    const buf = new ArrayBuffer(INSTANCE_BYTE_SIZE);
    const view = new DataView(buf);
    packInstance(instance, view, 0);
    const result = unpackInstance(view, 0);

    expect(result.sprite_id).toBeCloseTo(instance.sprite_id);
    expect(result.atlas_id).toBeCloseTo(instance.atlas_id);
    expect(result.screen_x).toBeCloseTo(instance.screen_x);
    expect(result.screen_y).toBeCloseTo(instance.screen_y);
    expect(result.z_order).toBeCloseTo(instance.z_order);
    expect(result.palette_id).toBe(instance.palette_id);
    expect(result.mask_flags).toBe(instance.mask_flags);
    expect(result.anim_frame).toBe(instance.anim_frame);
    expect(result.render_flags).toBe(instance.render_flags);
    expect(result.scale_x).toBeCloseTo(instance.scale_x);
    expect(result.scale_y).toBeCloseTo(instance.scale_y);
    expect(result.rotation).toBeCloseTo(instance.rotation, 4);
    expect(result.tint_r).toBe(instance.tint_r);
    expect(result.tint_g).toBe(instance.tint_g);
    expect(result.tint_b).toBe(instance.tint_b);
    expect(result.tint_a).toBe(instance.tint_a);
  });

  it("pack/unpack round-trip with default instance", () => {
    const buf = new ArrayBuffer(INSTANCE_BYTE_SIZE);
    const view = new DataView(buf);
    packInstance(DEFAULT_INSTANCE, view, 0);
    const result = unpackInstance(view, 0);

    expect(result.render_flags).toBe(1);
    expect(result.scale_x).toBeCloseTo(1.0);
    expect(result.scale_y).toBeCloseTo(1.0);
    expect(result.tint_r).toBe(255);
    expect(result.tint_g).toBe(255);
    expect(result.tint_b).toBe(255);
    expect(result.tint_a).toBe(255);
  });

  // ─── RenderInstanceBuilder ────────────────────────────────────────────────

  describe("RenderInstanceBuilder", () => {
    let builder: RenderInstanceBuilder;

    beforeEach(() => {
      builder = new RenderInstanceBuilder();
    });

    it("add and count track instances", () => {
      expect(builder.count()).toBe(0);

      builder.add({ ...DEFAULT_INSTANCE, sprite_id: 1 });
      expect(builder.count()).toBe(1);

      builder.add({ ...DEFAULT_INSTANCE, sprite_id: 2 });
      expect(builder.count()).toBe(2);
    });

    it("sort orders instances by z_order ascending", () => {
      builder.add({ ...DEFAULT_INSTANCE, z_order: 300, sprite_id: 3 });
      builder.add({ ...DEFAULT_INSTANCE, z_order: 100, sprite_id: 1 });
      builder.add({ ...DEFAULT_INSTANCE, z_order: 200, sprite_id: 2 });

      builder.sort();

      expect(builder.instances[0].z_order).toBe(100);
      expect(builder.instances[1].z_order).toBe(200);
      expect(builder.instances[2].z_order).toBe(300);

      // Verify the sprite_ids followed their z_orders
      expect(builder.instances[0].sprite_id).toBe(1);
      expect(builder.instances[1].sprite_id).toBe(2);
      expect(builder.instances[2].sprite_id).toBe(3);
    });

    it("clear empties the instance list", () => {
      builder.add({ ...DEFAULT_INSTANCE, sprite_id: 1 });
      builder.add({ ...DEFAULT_INSTANCE, sprite_id: 2 });
      expect(builder.count()).toBe(2);

      builder.clear();
      expect(builder.count()).toBe(0);
      expect(builder.instances).toHaveLength(0);
    });

    it("addDefault creates a visible instance with correct fields", () => {
      builder.addDefault(42, 100.5, 200.5, 500);

      expect(builder.count()).toBe(1);
      const inst = builder.instances[0];
      expect(inst.sprite_id).toBe(42);
      expect(inst.screen_x).toBe(100.5);
      expect(inst.screen_y).toBe(200.5);
      expect(inst.z_order).toBe(500);
      expect(inst.render_flags).toBe(1); // visible
      expect(inst.scale_x).toBe(1.0);
      expect(inst.scale_y).toBe(1.0);
      expect(inst.tint_r).toBe(255);
      expect(inst.tint_a).toBe(255);
    });

    it("toTypedArray produces correct byte size", () => {
      builder.addDefault(1, 0, 0, 0);
      builder.addDefault(2, 10, 20, 1);
      builder.addDefault(3, 30, 40, 2);

      const arr = builder.toTypedArray();
      expect(arr.byteLength).toBe(3 * INSTANCE_BYTE_SIZE);
    });

    it("toTypedArray produces empty array for empty builder", () => {
      const arr = builder.toTypedArray();
      expect(arr.byteLength).toBe(0);
      expect(arr.length).toBe(0);
    });

    it("fromTypedArray reconstructs instances from packed buffer", () => {
      builder.addDefault(10, 100, 200, 50);
      builder.addDefault(20, 300, 400, 150);

      const arr = builder.toTypedArray();
      const reconstructed = RenderInstanceBuilder.fromTypedArray(arr, 2);

      expect(reconstructed).toHaveLength(2);
      expect(reconstructed[0].sprite_id).toBeCloseTo(10);
      expect(reconstructed[0].screen_x).toBeCloseTo(100);
      expect(reconstructed[0].screen_y).toBeCloseTo(200);
      expect(reconstructed[0].z_order).toBeCloseTo(50);
      expect(reconstructed[1].sprite_id).toBeCloseTo(20);
      expect(reconstructed[1].screen_x).toBeCloseTo(300);
      expect(reconstructed[1].z_order).toBeCloseTo(150);
    });

    it("fromTypedArray round-trips through toTypedArray with full field fidelity", () => {
      const custom: RenderInstance = {
        sprite_id: 99,
        atlas_id: 5,
        screen_x: 640,
        screen_y: 480,
        z_order: 999,
        palette_id: 3,
        mask_flags: 0x0f,
        anim_frame: 7,
        render_flags: 1,
        scale_x: 1.5,
        scale_y: 0.75,
        rotation: 3.14159,
        tint_r: 128,
        tint_g: 64,
        tint_b: 32,
        tint_a: 255,
      };

      builder.add(custom);
      const arr = builder.toTypedArray();
      const result = RenderInstanceBuilder.fromTypedArray(arr, 1);

      expect(result[0].palette_id).toBe(3);
      expect(result[0].mask_flags).toBe(0x0f);
      expect(result[0].anim_frame).toBe(7);
      expect(result[0].render_flags).toBe(1);
      expect(result[0].tint_r).toBe(128);
      expect(result[0].tint_g).toBe(64);
      expect(result[0].tint_b).toBe(32);
      expect(result[0].tint_a).toBe(255);
      expect(result[0].scale_x).toBeCloseTo(1.5);
      expect(result[0].rotation).toBeCloseTo(3.14159, 4);
    });
  });
});
