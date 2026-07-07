self:
{ config, lib, pkgs, ... }:

let
  cfg = config.services.gewisprint;
  pkg = self.packages.${pkgs.system}.default;
in
{
  options.services.gewisprint = {
    enable = lib.mkEnableOption "GEWISprint CUPS print server";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkg;
      defaultText = lib.literalExpression "gewisprint flake package";
      description = "The gewisprint package to run.";
    };

    address = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      example = "0.0.0.0";
      description = "IP address to bind. Use 0.0.0.0 to expose on the LAN (no auth — keep it trusted).";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = "TCP port to listen on.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open {option}`port` in the firewall.";
    };

    enableCups = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable the local CUPS daemon (services.printing) that this server prints through.";
    };
  };

  config = lib.mkIf cfg.enable {
    services.printing.enable = lib.mkIf cfg.enableCups true;

    systemd.services.gewisprint = {
      description = "GEWISprint CUPS print server";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ] ++ lib.optional cfg.enableCups "cups.service";
      wants = lib.optional cfg.enableCups "cups.service";

      # lp/lpstat must be reachable at runtime.
      path = [ pkgs.cups ];

      environment = {
        IP = cfg.address;
        PORT = toString cfg.port;
        DIOXUS_PUBLIC_PATH = "${cfg.package}/share/gewisprint/public";
      };

      serviceConfig = {
        ExecStart = lib.getExe cfg.package;
        WorkingDirectory = "${cfg.package}/share/gewisprint";
        Restart = "on-failure";

        # Hardening. Printing goes through the CUPS unix socket, so no raw
        # device access is needed here.
        DynamicUser = true;
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ProtectControlGroups = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [ "@system-service" "~@privileged" "~@resources" ];
        # Only local + unix sockets (CUPS at /run/cups/cups.sock, HTTP listener).
        RestrictAddressFamilies = [ "AF_UNIX" "AF_INET" "AF_INET6" ];
      };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];
  };
}
