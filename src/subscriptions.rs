// Raydium
pub const RAYDIUM_SUBSCRIPTION: &str = r#"
subscription{
  Solana {
    Instructions(
      where: {
        Transaction: {
          Result: {
            Success: true
          }
        },
        Instruction: {
          Program: {
            Method: {
              is: "initialize2"
            },
            Address: {
              is: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
            }
          }
        }
      }
    ) {
      Block {
        Time
      }
      Transaction {
        Signature
      }
      Instruction {
        Accounts {
          Address
          IsWritable
          Token {
            Mint
            Owner
            ProgramId
          }
        }
        Logs
        Program {
          Method
          Name
          Arguments {
            Type
            Name
            Value {
              __typename
              ... on Solana_ABI_Integer_Value_Arg {
                integer
              }
              ... on Solana_ABI_String_Value_Arg {
                string
              }
              ... on Solana_ABI_Address_Value_Arg {
                address
              }
              ... on Solana_ABI_BigInt_Value_Arg {
                bigInteger
              }
              ... on Solana_ABI_Bytes_Value_Arg {
                hex
              }
              ... on Solana_ABI_Boolean_Value_Arg {
                bool
              }
              ... on Solana_ABI_Float_Value_Arg {
                float
              }
              ... on Solana_ABI_Json_Value_Arg {
                json
              }
            }
          }
          AccountNames
          Address
        }
      }
    }
  }
}
"#;



// Pumpswap
pub const PUMPSWAP_SUBSCRIPTION: &str = r#"
subscription {
  Solana {
    Instructions(
      where: {Instruction: {Program: {Method: {is: "create_pool"}, Address: {is: "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"}}}}
    ) {
      Instruction {
        Program {
          Address
          Name
          Method
          Arguments {
            Name
            Type
            Value {
              ... on Solana_ABI_Json_Value_Arg {
                json
              }
              ... on Solana_ABI_Float_Value_Arg {
                float
              }
              ... on Solana_ABI_Boolean_Value_Arg {
                bool
              }
              ... on Solana_ABI_Bytes_Value_Arg {
                hex
              }
              ... on Solana_ABI_BigInt_Value_Arg {
                bigInteger
              }
              ... on Solana_ABI_Address_Value_Arg {
                address
              }
              ... on Solana_ABI_String_Value_Arg {
                string
              }
              ... on Solana_ABI_Integer_Value_Arg {
                integer
              }
            }
          }
          AccountNames
          Json
        }
        Accounts {
          Address
          IsWritable
          Token {
            Mint
            Owner
            ProgramId
          }
        }
        Logs
        BalanceUpdatesCount
        AncestorIndexes
        CallPath
        CallerIndex
        Data
        Depth
        ExternalSeqNumber
        Index
        InternalSeqNumber
        TokenBalanceUpdatesCount
      }
      Transaction {
        Fee
        FeeInUSD
        Signature
        Signer
        FeePayer
        Result {
          Success
          ErrorMessage
        }
      }
      Block {
        Time
      }
    }
  }
}
"#;


// Meteora
pub const METEORA_SUBSCRIPTION: &str = r#"
subscription MyQuery {
  Solana {
    Instructions(
      where: {Transaction: {Result: {Success: true}}, Instruction: {Program: {Method: {is: "initializePermissionlessConstantProductPoolWithConfig2"}, Address: {is: "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB"}}}}
    ) {
      Block {
        Time
      }
      Instruction {
        Accounts {
          Address
          IsWritable
          Token {
            Mint
            Owner
            ProgramId
          }
        }
        Program {
          AccountNames
          Address
          Arguments {
            Name
            Type
            Value {
              ... on Solana_ABI_Integer_Value_Arg {
                integer
              }
              ... on Solana_ABI_String_Value_Arg {
                string
              }
              ... on Solana_ABI_Address_Value_Arg {
                address
              }
              ... on Solana_ABI_BigInt_Value_Arg {
                bigInteger
              }
              ... on Solana_ABI_Bytes_Value_Arg {
                hex
              }
              ... on Solana_ABI_Boolean_Value_Arg {
                bool
              }
              ... on Solana_ABI_Float_Value_Arg {
                float
              }
              ... on Solana_ABI_Json_Value_Arg {
                json
              }
            }
          }
          Method
          Name
        }
      }
      Transaction {
        Signature
        Signer
      }
    }
  }
}
"#;



// Orca 

// pub const ORCA_SUBSCRIPTION: &str = r#"
// subscription {
//   Solana {
//     Instructions(
//       where: {
//         Instruction: {
//           Program: {
//             Method: {in: ["initializePool", "initializePoolV2"]}
//             Address: { is: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" }
//           }
//         }
//       }
//     ) {
//       Instruction {
//         Program {
//           Method
//           Arguments {
//             Name
//             Value {
//               __typename
//               ... on Solana_ABI_Integer_Value_Arg {
//                 integer
//               }
//               ... on Solana_ABI_String_Value_Arg {
//                 string
//               }
//               ... on Solana_ABI_Address_Value_Arg {
//                 address
//               }
//               ... on Solana_ABI_BigInt_Value_Arg {
//                 bigInteger
//               }
//               ... on Solana_ABI_Bytes_Value_Arg {
//                 hex
//               }
//               ... on Solana_ABI_Boolean_Value_Arg {
//                 bool
//               }
//               ... on Solana_ABI_Float_Value_Arg {
//                 float
//               }
//               ... on Solana_ABI_Json_Value_Arg {
//                 json
//               }
//             }
//           }
//         }
//         Accounts {
//           Address
//         }
//       }
//       Transaction {
//         Signature
//       }
//     }
//   }
// }
// "#;


pub const COMBINED: &str = r#"
subscription {
  Solana {
    Instructions(
      where: {Instruction: {Program: {Method: {in: ["initialize2", "create_pool", "initializePermissionlessConstantProductPoolWithConfig2"]}, Address: {in: ["675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA", "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB"]}}}}
    ) {
      Block {
        Time
      }
      Instruction {
        Program {
          Name
          Method
          Arguments {
            Name
            Value {
              __typename
              ... on Solana_ABI_Integer_Value_Arg {
                integer
              }
              ... on Solana_ABI_String_Value_Arg {
                string
              }
              ... on Solana_ABI_Address_Value_Arg {
                address
              }
              ... on Solana_ABI_BigInt_Value_Arg {
                bigInteger
              }
              ... on Solana_ABI_Bytes_Value_Arg {
                hex
              }
              ... on Solana_ABI_Boolean_Value_Arg {
                bool
              }
              ... on Solana_ABI_Float_Value_Arg {
                float
              }
              ... on Solana_ABI_Json_Value_Arg {
                json
              }
            }
          }
          AccountNames
          Address
        }
        Accounts {
          Address
        }
      }
      Transaction {
        Signature
      }
    }
  }
}
"#;
