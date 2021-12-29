const cdk = require("@aws-cdk/core");
const {
  GithubActionsIdentityProvider,
  GithubActionsRole,
} = require("aws-cdk-github-oidc");
const dynamo = require("@aws-cdk/aws-dynamodb");
const { CfnOutput } = require("@aws-cdk/core");

class CdkStack extends cdk.Stack {
  /**
   *
   * @param {cdk.Construct} scope
   * @param {string} id
   * @param {cdk.StackProps=} props
   */
  constructor(scope, id, props) {
    super(scope, id, props);

    const provider = GithubActionsIdentityProvider.fromAccount(
      this,
      "GitHubProvider"
    );

    const statsTable = new dynamo.Table(this, "ESMCheckerDynamoStatsTable", {
      billingMode: dynamo.BillingMode.PAY_PER_REQUEST,
      partitionKey: {
        name: "timestamp",
        type: dynamo.AttributeType.NUMBER,
      },
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    cdk.Tags.of(statsTable).add("project", "esm-checker");

    const dynamoRole = new GithubActionsRole(this, "ESMCheckerDynamoRole", {
      provider,
      owner: "lannonbr",
      repo: "esm-checker",
      filter: "ref:refs/heads/main",
    });

    cdk.Tags.of(dynamoRole).add("project", "esm-checker");

    const siteReadRole = new GithubActionsRole(this, "ESMCheckerSiteReadRole", {
      provider,
      owner: "lannonbr",
      repo: "esm-checker-site",
      filter: "ref:refs/heads/main",
    });

    cdk.Tags.of(siteReadRole).add("project", "esm-checker");

    statsTable.grantReadWriteData(dynamoRole);
    statsTable.grantReadData(siteReadRole);

    const tableName = new CfnOutput(this, "ESMCheckerDynamoStatsTableName", {
      value: statsTable.tableName,
    });
  }
}

module.exports = { CdkStack };
