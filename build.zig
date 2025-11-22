const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const llvm = b.option(bool, "llvm", "Use LLVM backend.") orelse false;

    const zluajit = b.dependency("zluajit", .{
        .target = target,
        .optimize = optimize,
        .@"lua52-compat" = true,
        .llvm = llvm,
    });
    const zev = b.dependency("libzev", .{
        .target = target,
        .optimize = optimize,
        .llvm = llvm,
    });

    const lib = b.addLibrary(.{
        .name = "allelua",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/root.zig"),
            .target = target,
            .optimize = optimize,
        }),
        .linkage = .static,
        .use_llvm = llvm,
    });
    lib.root_module.addImport("zluajit", zluajit.module("zluajit"));
    lib.root_module.addImport("zev", zev.module("zev"));
    b.installArtifact(lib);

    const exe = b.addExecutable(.{
        .name = "allelua",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    exe.root_module.addImport("allelua", lib.root_module);
    b.installArtifact(exe);

    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);
    const lib_unit_tests = b.addTest(.{ .root_module = lib.root_module });
    const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);

    const exe_unit_tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    const run_exe_unit_tests = b.addRunArtifact(exe_unit_tests);

    const install_lib_unit_tests = b.addInstallArtifact(lib_unit_tests, .{});

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_lib_unit_tests.step);
    test_step.dependOn(&run_exe_unit_tests.step);
    test_step.dependOn(&install_lib_unit_tests.step);
}
