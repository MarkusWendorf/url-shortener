import * as cdk from "aws-cdk-lib";
import * as ec2 from "aws-cdk-lib/aws-ec2";
import * as elb from "aws-cdk-lib/aws-elasticloadbalancingv2";
import * as cloudfront from "aws-cdk-lib/aws-cloudfront";
import * as origins from "aws-cdk-lib/aws-cloudfront-origins";
import * as targets from "aws-cdk-lib/aws-elasticloadbalancingv2-targets";
import { Construct } from "constructs";

export class InfraStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const vpc = new ec2.Vpc(this, "Vpc", {
      maxAzs: 2,
      natGateways: 0,
      subnetConfiguration: [
        {
          cidrMask: 24,
          name: "public",
          subnetType: ec2.SubnetType.PUBLIC,
        },
      ],
    });

    const securityGroup = new ec2.SecurityGroup(this, "SecurityGroup", { vpc });
    securityGroup.addIngressRule(
      ec2.Peer.anyIpv4(),
      ec2.Port.allTcp(),
      "All Open IPv4"
    );

    securityGroup.addIngressRule(
      ec2.Peer.anyIpv6(),
      ec2.Port.allTcp(),
      "All Open IPv6"
    );

    const key = new ec2.KeyPair(this, "SshKey");

    const instance = new ec2.Instance(this, "Machine", {
      instanceType: ec2.InstanceType.of(
        ec2.InstanceClass.T3A,
        ec2.InstanceSize.MEDIUM
      ),
      blockDevices: [
        {
          deviceName: "/dev/xvda",
          volume: ec2.BlockDeviceVolume.ebs(50, {
            volumeType: ec2.EbsDeviceVolumeType.IO2,
            iops: 3200,
          }),
        },
      ],
      machineImage: ec2.MachineImage.latestAmazonLinux2023(),
      vpc,
      keyPair: key,
      securityGroup,
      init: ec2.CloudFormationInit.fromElements(
        ec2.InitService.systemdConfigFile("url-shortener", {
          command: "/var/url-shortener/url-shortener",
          cwd: "/var/url-shortener",
        }),
        ec2.InitService.enable("url-shortener", {
          serviceManager: ec2.ServiceManager.SYSTEMD,
        })
      ),
    });

    const alb = new elb.ApplicationLoadBalancer(this, "ELB", {
      vpc,
      securityGroup,
      internetFacing: true,
    });

    const listener = alb.addListener("Listener", {
      protocol: elb.ApplicationProtocol.HTTP,
    });

    listener.addTargets("Ec2Target", {
      protocol: elb.ApplicationProtocol.HTTP,
      targets: [new targets.InstanceTarget(instance, 3333)],
    });

    // TODO: https + certificates
    const cf = new cloudfront.Distribution(this, "CDN", {
      defaultBehavior: {
        origin: new origins.LoadBalancerV2Origin(alb, {
          protocolPolicy: cloudfront.OriginProtocolPolicy.HTTP_ONLY,
        }),
        originRequestPolicy:
          cloudfront.OriginRequestPolicy.ALL_VIEWER_AND_CLOUDFRONT_2022,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_ALL,
      },
    });

    new cdk.CfnOutput(this, "Cloudfront", {
      value: cf.distributionDomainName,
    });
  }
}

const app = new cdk.App();
new InfraStack(app, "UrlShortener", {});
