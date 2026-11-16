{
  description = "Ely Office: offline desktop apps for documents, sheets, slides, PDF, Git and Markdown";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      lib = nixpkgs.lib;
      # Written by packaging/nix.sh from a release's manifests.
      release = lib.importJSON ./nix/release.json;
      app =
        slug: info:
        let
          pname = slug;
          inherit (release) version;
          src = pkgs.fetchurl { inherit (info) url sha256; };
          contents = pkgs.appimageTools.extractType2 { inherit pname version src; };
        in
        pkgs.appimageTools.wrapType2 {
          inherit pname version src;
          # GPUI loads these by name.
          extraPkgs =
            pkgs: with pkgs; [
              vulkan-loader
              wayland
              libxkbcommon
            ];
          extraInstallCommands = ''
            install -Dm444 ${contents}/${slug}.desktop -t $out/share/applications
            install -Dm444 ${contents}/${slug}.png -t $out/share/icons/hicolor/512x512/apps
          '';
          meta = {
            description = "${info.name}, from Ely Office ${version}";
            homepage = "https://github.com/ELY-Open-Source/ElyOffice";
            license = map lib.getLicenseFromSpdxId (lib.splitString " AND " info.license);
            sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
            platforms = [ system ];
            mainProgram = slug;
          };
        };
      apps = lib.mapAttrs app release.apps;
    in
    {
      packages.${system} = rec {
        ely-office = pkgs.symlinkJoin {
          name = "ely-office-${release.version}";
          paths = lib.attrValues apps;
          meta.license = lib.unique (lib.concatMap (each: each.meta.license) (lib.attrValues apps));
        };
        default = ely-office;
      };
    };
}
