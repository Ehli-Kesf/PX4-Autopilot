include(FetchContent)

if(NOT PLATFORM STREQUAL "posix")
	set(Rust_CARGO_TARGET "thumbv7em-none-eabihf")
	message("OLAA ERZURUM PLATFORM ${PLATFORM} ve STM32 ye derlendi!!")
endif()

FetchContent_Declare(
Corrosion GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git GIT_TAG v0.5 # Optionally specify a commit hash, version tag or branch here
)
# Set any global configuration variables such as `Rust_TOOLCHAIN` before this line!
FetchContent_MakeAvailable(Corrosion)

# Import targets defined in a package or workspace manifest `Cargo.toml` file
corrosion_import_crate(MANIFEST_PATH Cargo.toml NO_STD PROFILE release)
corrosion_set_env_vars(px4_simple_app DENEME=hello)

# corrosion_link_libraries(px4_simple_app uORB)

px4_add_module(
	MODULE examples__px4_simple_app
	MAIN px4_simple_app
	SRCS
		px4_simple_app.c
	DEPENDS
	)
target_link_libraries(examples__px4_simple_app PUBLIC px4_simple_app)
