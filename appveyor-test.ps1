if ($env:CONFIGURATION -eq 'release') {
    # TODO: These "arguments" might get passed as a
    # single argument if there are multiple. Read up on
    # `Start-Process` etc. to figure out how to do this properly.
    $cargoargs = '--release'
}
cargo test --all $cargoargs
