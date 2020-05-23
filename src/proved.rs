//
// Copyright 2019 Tamas Blummer
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
//!
//! # SPV proved transaction
//!
//!

use bitcoin::hashes::{sha256d, Hash, HashEngine};
use bitcoin::{BitcoinHash, Block, Transaction};

/// A confirmed transaction with its SPV proof
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProvedTransaction {
    transaction: Transaction,
    merkle_path: Vec<(bool, sha256d::Hash)>,
    block_hash: bitcoin::BlockHash,
}

impl ProvedTransaction {
    pub fn new(block: &Block, txnr: usize) -> ProvedTransaction {
        let transaction = block.txdata[txnr].clone();
        ProvedTransaction {
            block_hash: block.header.bitcoin_hash(),
            merkle_path: Self::compute_proof(txnr, block),
            transaction,
        }
    }
    /// get a copy of the transaction
    pub fn get_transaction(&self) -> Transaction {
        self.transaction.clone()
    }

    pub fn get_block_hash(&self) -> &bitcoin::BlockHash {
        &self.block_hash
    }

    /// compute the merkle root implied by the SPV proof
    pub fn merkle_root(&self) -> bitcoin::TxMerkleNode {
        self.merkle_path.iter().fold(
            bitcoin::TxMerkleNode::from_inner(self.transaction.txid().into_inner()),
            |a, (left, h)| {
                let mut encoder = bitcoin::TxMerkleNode::engine();
                if *left {
                    encoder.input(&h[..]);
                    encoder.input(&a[..]);
                } else {
                    encoder.input(&a[..]);
                    encoder.input(&h[..]);
                }
                bitcoin::TxMerkleNode::from_engine(encoder)
            },
        )
    }

    /// compute a proof for a transaction in a block
    /// panics if transaction is not in the block
    pub fn compute_proof(mut track: usize, block: &Block) -> Vec<(bool, sha256d::Hash)> {
        /// one step of the reduction to merkle root
        /// it returns the reduced vector and also the operation applied to the tracked id
        /// the operation is (left, hash) left is true if hash should be hashed before the tracked id
        fn binhash(
            hashes: &[sha256d::Hash],
            track: usize,
        ) -> (Vec<sha256d::Hash>, Option<(usize, bool, sha256d::Hash)>) {
            let mut result = Vec::new();
            let mut op = None;
            if hashes.len() > 1 {
                for (i, pair) in hashes.chunks(2).enumerate() {
                    let mut engine = sha256d::Hash::engine();
                    if pair.len() == 1 {
                        if track == i * 2 {
                            op = Some((i, false, pair[0]));
                        } else if track == i * 2 + 1 {
                            op = Some((i, true, pair[0]));
                        }
                        engine.input(&pair[0][..]);
                        engine.input(&pair[0][..]);
                    } else {
                        if track == i * 2 {
                            op = Some((i, false, pair[1]));
                        } else if track == i * 2 + 1 {
                            op = Some((i, true, pair[0]));
                        }
                        engine.input(&pair[0][..]);
                        engine.input(&pair[1][..]);
                    }
                    result.push(sha256d::Hash::from_engine(engine));
                }
            }
            (result, op)
        }

        let mut ids = block
            .txdata
            .iter()
            .map(|t| t.txid().as_hash())
            .collect::<Vec<_>>();
        let mut proof = Vec::new();
        while let (i, Some((t, left, hash))) = binhash(ids.as_slice(), track) {
            proof.push((left, hash));
            ids = i;
            track = t;
        }
        proof
    }
}

#[cfg(test)]
mod test {
    use bitcoin::hashes::hex::FromHex;
    use bitcoin::{consensus, Block};

    use super::*;

    #[test]
    pub fn test_spv_proof() {
        let blockdump="000040207434e9f3762bf60b4c8430bffbfb6bb193644b653f5a0f0000000000000000008a15edb49e2fe9af06ff1ed2dc80af9ef3e33e9e22a5638b15c191543bb2aa902f05345d9b0d1f17dd02f3542c010000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff5e0363f2081b2f5669614254432f4d696e65642062792070616e3838383838382f2cfabe6d6d19a95396e8b4519ff8b9abfed0fd714e83818be4b2028033cabdb600b4f5ede91000000000000000108189e903eaf273dcd72eb8808ed70000ffffffff0204e48a4a000000001976a914536ffa992491508dca0354e52f32a3a7a679a53a88ac0000000000000000266a24aa21a9edf884c4f8d82aac50534009aa7f75e9b4cb4d95babf308bd9914daf3c2fc73a1d012000000000000000000000000000000000000000000000000000000000000000000000000001000000017da3385dd237ffd2e6ca57254e0fd390cab622f19fd1ffe29f24b9fd56526ce8010000006b483045022100d4678ee5869c7f1ff41728d57de2a5c5bf4cb4e9702668e62d3b71cdef73843d022071b95eb52b9f028b961d79e7f7037c4c50306fb536f1429a33947468c5b34e5f012103944b0a8020a1239b8d056cd70f618457adf44061bd5dc2746db6ea95e4c5ac89ffffffff02201b47000000000017a914373b07f02a3c5ea17000d8d8db934576ccc7d28087401d2a00000000001976a914bc5c884908ad739b11e5144fa0373c9dc3bbaf0b88ac000000000100000001ffb19b52f3448e82b4aee220c859dbc4cb9c2651b16105fd6e8b9398e403d629000000006a47304402205c14d922536c32514c74c36a7f8525f5509a03552d4e1ef9d47ad893c5db986c02200e48747077f837fc6e402f067cb9f1a3021a15385e9db6b1d5162742645aef7b012103c0be0c2f24fe4c2c1c3aa08672bdc8fe155b594268fd6d28defbde4f29c13fcbfeffffff030000000000000000166a146f6d6e69000000000000001f00000004438a344022020000000000001976a9143d55424c15dea666edbfccdde5352192e212ab4b88ac7e1de600000000001976a9144a26c25c1b474f2a348966e328ce6f4e84403e0e88ac61f208000100000001a153fdaf72b25efc3ff9013483e2e33b7ab8e9c1bf14e5e7575d7d3424f9ff83000000006a47304402202248b8ba13947661dc6823bd5cef740404a268d45dbf089c25c69f1bd446b72302207ccfbbc6dee0b80f093a7d0a586f804636014f542388b86c67ed128b7a85fdb101210232151d6bc317ab0b458dabb6c1c4046c9e451cbbd0de7247b56f301ac7e9f6aeffffffff0390203f00000000001976a91428f923d53144f9ae8aec597865286d4c6fe4dd0e88ac0000000000000000166a146f6d6e69000000000000001f00000000cfa3992c1c0200000000000017a91454992e62a1d0c61fa9318534477d288a8d9e8fd6870000000001000000012a5bb29f35d1252e8897e37ca48ff9de32f7872ee0ef82e606d12b0b7e118c21000000006a4730440220182a119452d34e93d6256245a654c3f7b99afc86c9cb3213ae5e86442231f42d022045a392f61bfa57832425938944078151a896f82cebc5df79412ccad1fc99973e0121030651e1d15ae9a284ffd712885529d3344db3700be756e6c22c56a6c1b57d359dffffffff0324640900000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac220200000000000017a91436d2ddb6a53c7d6d4f00e919353902ad9f71314d870000000000000000166a146f6d6e69000000000000001f000000217d241f000000000001000000012b5227c6726e07aae3f8158e9f0a639f3f20e1cabde9ceebc477c8a8534b56b8000000006b48304502210096a795859901c5548c36a216688e47663dff8380993cb016bbd6f59f4a46195002202f25b43415bf1a6920ac624689dfb462eb4c254d5b7a76c579a4cb53f3cef2620121030651e1d15ae9a284ffd712885529d3344db3700be756e6c22c56a6c1b57d359dffffffff0323640900000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac220200000000000017a914463fe2b5f49843fdeef1d88357f9298e25063e37870000000000000000166a146f6d6e69000000000000001f0000003f9bfd2c00000000000100000001fab65101f020c3dae032527351df319b184936c5d56c140166543a6170ed4b0e000000006a473044022063ca66a5fda2e6758b97ccb799e3e199862d9a85c41b682fcd3139abd149053102206858e79d0b19b2b74102ae1a1909ef4eb20cc79ae79e5baa0f08117d7ff018b60121030651e1d15ae9a284ffd712885529d3344db3700be756e6c22c56a6c1b57d359dffffffff0324640900000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac22020000000000001976a914db76df46a12d6c95922bbadabe63a04ef07c804988ac0000000000000000166a146f6d6e69000000000000001f00000043141d66c0000000000100000001407c9ce88691020d748a46c1beeab4e9f7162c772f12793cbc8b04554baeb541000000006a473044022010fac346f94bc66e1ec014c7749fde0d639f9fa148a27b36fd539a524993d90f022035659be4b28e6813e66ffdf4bef29676d9a17bf779aa015ebc85c94cddc104890121030651e1d15ae9a284ffd712885529d3344db3700be756e6c22c56a6c1b57d359dffffffff0322640900000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac22020000000000001976a9149944ef45a177f84923b1e5678c1932da76a070d388ac0000000000000000166a146f6d6e69000000000000001f00000009a997bf000000000001000000011685132408d61088e5aff6c079e009d25f1ecd30428ab2bb55ea53aafcbbe76b000000006a4730440220762d5d55f58b2047af16aa883cbb5907943a97f70af92b7b88c3db70dd49d616022067f6e77200908c980083fc9fe36e9644d2ac7cc759ff0d6d9f8edaa7fdfffd580121030651e1d15ae9a284ffd712885529d3344db3700be756e6c22c56a6c1b57d359dffffffff0323640900000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac22020000000000001976a91401d6d89ec35b48a61a74bcadf96024d3ceef39f288ac0000000000000000166a146f6d6e69000000000000001f000000140ac5ef9000000000010000000188cff42a6481a6070596c81972fbdb54c51d1e9dbe385399b1735b1473eafe5d010000006b483045022100bbe90f4f4b31c828d08547c9b0aa753dd3e6d6c6d47bc344205df7e8dcd53e9202203a97de8b8214ae66cf3dea3e55b7950969d160e8814039d1cd08d896dd5c5e4e012102ef6c624b482b4e1ca69d925145b3ce01adb88b99540601731bc3b47ad76d4b8effffffff0248b00200000000001976a914f6fb6d68ba3f8f1477964afaa31ab73c388e519388ac38a21501000000001976a914ea1b43049393b07a90948c8c3afafb0f941b781188ac000000000100000002f74bf33afbcbdc5bc1bb2ffb6d05c8a6f48025987c019842276d237991378c85010000006a473044022052392d327254f117cffa72e93b06d6c7b6123379cf2ecb0d33ef95d0b263c53602203a311673986929237162d07f63bac3d917745afdfa188a894fdc49bcf8bee3630121030cc0dcd3453b1ee72ddb7161dc6ddf006c236349809eb0b13c8e8854d0e6f56affffffff9cc7860de73401576367fab966895b2a77a43dbc7b409f01049f642dd91f60b1000000006b483045022100ab5e48a7148572e3b2557fb4937cb7ca0db936c5a5644cb28fc5f996bcbd1b0a02205e00e3ce3253627a3bdf590dd8495bf2ca297295abffb070e903675d2f74562d0121030651e1d15ae9a284ffd712885529d3344db3700be756e6c22c56a6c1b57d359dffffffff0349f90000000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac22020000000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac0000000000000000166a146f6d6e69000000000000001f0000003356d2fd000000000001000000026551b01861071531860893bde2375ee5533bdce9a33304b5168512fb1462fbfd020000006a47304402200c93e587ca775f541bbb6e7c6c0853971e19f11cf3a9034f3a2a7d993ac39e45022017d6e482ee38de8be10741d35a444bc8c0169d22e619b2275e5bfb1b7c24b04c012103b62faf3ab98d24e02e11766e0fd321234817ddfe505b5ef9e5f89f36c1807f5affffffff31981f80b0b6e2d4d893472039126ef3a4b6099f3f221ba459f19eb45b0c0f34000000006b483045022100f7c8c398e2b2b34d5c9397b64350a103e66170be78313f59fba6c579c77220b802204c652d76d5736f74065ffba46260e36b575fd3d58d352704ccba7a5c7c4314e60121030651e1d15ae9a284ffd712885529d3344db3700be756e6c22c56a6c1b57d359dffffffff034bf90000000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac22020000000000001976a914b64513c1f1b889a556463243cca9c26ee626b9a088ac0000000000000000166a146f6d6e69000000000000001f0000003092b89300000000000100000001a2343be86448f93bebb900d653c202a71aeea361f1eab3ca0d88a7c0b92d8c9f010000006a473044022050892c8cae1a74f225399fd67a139c360209d3dbf793fd3d281209da3cbdd0ef02200de4e8aed5621dd8461f3f208b445c060841caa393b63674cd1005db4fe408630121034eac30c603f2c1df608b63739b5386726f6cd1cda40a969a15fb2f61da507d31feffffff030000000000000000166a146f6d6e69000000000000001f00000015aeafd0e522020000000000001976a91474304a69edaab12e6aa98cb22338354887a1df2188ac9d3b8500000000001976a91412aed4a2fed565f0f473b3688e60246576710f2a88ac61f20800010000000278b319fcf6b5e3e5f7815a0b0d9f33fce02efff2939971a5f5221a9e939124c5020000006a47304402207373e21cbd23b1cd94f72a9f3b4be112e19eab357e603bbdf9580abeb69b1fb702204219ded223c674a35e501f1064ba7b8f3f81b9972af92ba34b8784a1e0a142cc01210309ea6d7179f7d4c27dea7e67a86f2f5037a2d4b54a84ee521bd8655b40f93671fffffffff96a118547e374ab2243925c7566f6c1877c055bff2cad1c02beecf83beccb12000000006b483045022100a2d1b335d5cf8ff70d9ef7e43c7a88b5a3494f0b444957a5764704a8eb5e3789022050df35a308172dde1df7b64ae13a30947a172dc92fcf0a23f90e681e9b54e4d1012102d9ab13df1b9a2eb681397545928edb2f965181bd2e4c8d62b1de4ad30a0fdf89feffffff0369999f02000000001976a914818a8a591862ad9ba62526d721f5573dd4c6d4a888ac0000000000000000166a146f6d6e69000000000000001f0000000277cf2a0022020000000000001976a914818a8a591862ad9ba62526d721f5573dd4c6d4a888ac0000000001000000017954034837e2fe2df04669a76af11975dc035468d5817ef50f23b50ed6fbd234000000006b483045022100ab280fd50e4a0e9b2f1cc621d83042e9ffd90f5f6fe1359e525795f54c6ac08902203a8f038d63efdcdfe77a248f9d8c9f9be4c244a7e56e6ff5e9e95d8c9807f2bd012102a9b25030614618a4dc6373b5ab1d8b0b785f14e5231d8320302a6179f5e33cdbffffffff03aa4ab901000000001976a9141d294618ae97828821ad8601e47ebf47e07db0ba88ac0000000000000000166a146f6d6e69000000000000001f0000003651c37d0022020000000000001976a9145d9d8a782114dd46a3430dc0f60c23ceaaac680588ac0000000001000000018156c4bc1706c3d2e0d03afc69b5416e60463a2c5d725e98d646fb3cffda877f000000006b483045022100b0c5e7181720a72a1e02591f405b2fc33ef9100803271a10ad41f92c3629ec3e02205115802b19706b661423424d1db9d07ca7ca0239d7200c456428579b350b0c50012102a9b25030614618a4dc6373b5ab1d8b0b785f14e5231d8320302a6179f5e33cdbffffffff031909b901000000001976a9141d294618ae97828821ad8601e47ebf47e07db0ba88ac0000000000000000166a146f6d6e69000000000000001f00000023284d260022020000000000001976a914b4917b4329ef1ee7c7d261a13a877c40578645cb88ac0000000001000000000102a4e361e495f54876c3e22bec3d35a3b953f628331425037d30186d31ff7d4afa000000006a4730440220376f9de8d058e9910ab0f8227aaa1de568ffab3df57da50de63de8bd31c0babb02203da83bd33d8295c145565fbe428e45ca33e54ceeaa7a3888101a245fddd4e8c5012103bc242c0b586ef160b678d05292e4f8e59a98a6de658e8073613d7af3a2d533ddffffffffa7b6fdd10979b0f96956fd75fe741cb57e6dde74016ae96a6568b0868248b63d05000000171600144778280af8fd7203ea1db5bc38247abd218277eeffffffff014ba40a00000000001976a914526f9198b9ee60ea9ac06bc17333703143e2f38288ac000247304402204ef8d00eab5afee1b9e86a73e30885ca9d687d25bf85e60a8644d69ef3ca7e7502200ca629abe20cf5757f769e52dea9b0c0cf47dcee0bb50f5f532a2ea4771a366b0121029d0e49b6a2de51dbcd971d5740c72bc678c0867c2983f87b480db8fb99d54318000000000100000001763b2c4e8596ca1bdbd38a873629d41838ca6f2d6fea42f9447575ad7d42a3d2010000006b483045022100cc28ae1a0c9ab64e73f5e7e95d997311086fb820f5f0a895dcf6c6cceab9ee1a0220363672c8e37b7843a9ff0d835c4e398b81d5ceeba50505a1b361c6868f7e67810121023b66650aeb4ba2264aa9cf9690b0d6bbb73ca42d0183220482b838116a50e645ffffff00028084307a0000000017a9148aaf08633f335f425e0606c50b6b280e6f051bb387ca680e00000000001976a914fb4ce5ab55c13c5f8b89f839dd470b4e415381ec88ac000000000100000000010198a3a07bdf8f1a33cd008280d7cc0e5148d7b1b9f1f3fd7c32945454d3b43ece0000000000ffffffff0243730400000000001976a91443d21e9e618f9ff20e034ec50de12e52dd59c2cb88ac63ca6c0000000000160014202add65e3f81bb4901e1b07d23869d894d94e4c02483045022100d84204ff846485f81947db9ebd71881449b4656803a98d110b166f5c9428269702206adfc91424e74e93ad66bef69c7bfb82eece98908250d6a1bacba9263c0ad1b40121038737412cbdb454e0c53e023ec87408217123bf4b9ac53d7243363d41aad43d76000000000100000000010162b708565b1583ad65f33a5a9fedf511c684adc72f206f171371928bba14fe3a0000000000ffffffff0222dc000000000000160014b7f9fe4106c3634b163eedb4893209d6d4d276fbe6de0200000000001976a9147d106f302b42520f6408ef6fb5dfb22174ccdd9e88ac024730440220168bff1046323370083c69f0156ae51e3e099ce19b8ef5ac4d036c9bc22efe4b022047478d089142f5d0d99b78c33585eabede9ead0de773557c2206e670aa1816fa01210361f9027debb6947affceda88cb7e35ca9408a85a5614bad0c47199275d44699300000000010000000001018cfdde1a7c2442bf7c2e12cd3fcebc65a6a35ae8d3a97390f50b6e9c86611e7400000000171600147dbbda790244753aca16a0854bdf31becfe0e6abffffffff02084701000000000017a9144d7bda6683f6a636594d5f65c70192d16a75c3ce876042000000000000160014ff5949074d3b6d91fd424ce3d51cf167ff0b0d4f024730440220260169a99f80a6bffc2eb032ec33033debc851b945203626ee54b6d02fa3d655022031ce734239e719d056b994b8a373e0796dffc623173818063f0936d488d9e4a50121021149714966976a4a9efe186fcbf4433873e2dbd5edaa284ee891ba3aa6c08dbc0000000001000000000101beceb3dff4f96c0d778602cfce5561848961f57c49dfa7ccb9a4899085d0386f2b00000017160014c26515f5a900521e2f8ea684a78aedcdcb826a07ffffffff02387c0000000000001600147672630c0c78e51cf7a106c28bd17b4424e26d9aed7301000000000017a914f3143ec2c6374bd72ca9f71ed5bdde969aa44c638702483045022100e6750190f9976a118ac57ab3057ec3defd3e8c2e5e8d8c2c8d5b437b220d019902206c333b6535ec024e8b44f16a926137935ca48ea6d5038cb8c0bf2d0a613aef3501210340bbf2410717a6da90db27517e939085f2f0b00b6131665f63f4e332ffb029ba0000000001000000000101218961ab1a2e569a0afd97a7b654acf29fb200866f66b3f5fbfb7b7f3fd0baf00100000000ffffffff02f1e1510000000000160014b7d4e30603c532fc309c25e3a6c0a232ffacf07f18a32b000000000017a91453abc3d98ef4908f3d5bbad4ac1a02c72286673f8702483045022100fac676ea6750d1e9bafb8d21bbd45b5505f6a7bdc3774b608ed07caf231e9e5502206fd737e0dcf64721c1d08b8b945daf5a281ee703a5f0d03f7f86d2c302ab71330121022ea98693601f17e5a9a89ea1e91fefdb1f8dad51a6fc9d35d22307824a2b9c920000000001000000000101fcb62063565a10f9a56c5f12de82d7fa56950a795f02bea5eee9dd09242f7bd584000000171600142b07729fa68593306fe1459ff9fde64ba5896266ffffffff02421b010000000000160014a6a32dbafeec6aa22b1b64a343ca2d23e5e9df76bc910200000000001976a91414bfcb812805cae80485bb71a21c544dbb619ba288ac024730440220311ba4cec594263f5d6c6d570cba6dae0799062cf06664aac3de587b51493a5102204ab9abb3cd2d375e09683e1e493d5ef61349a26ce16847571706e9b3620acb00012103fe8d2280283ec3af0218f4545a25f27c28a7e96164565a9d810894be71cd82970000000002000000000101671e699f5dcfc390272cea51492b5b481003ec31d59168c067ab82f1f467525f030000001716001460f3f353b3f02bb1926071c85a3b9f0c36eb7adafeffffff0e334f551b0000000017a914b331c9b4000448c72fb2e9155d649996ff4f670587006b03000000000017a914ee251cb45f0f7eed1de34e03f6359fe715ee825187f6da03000000000017a914f21419255dd52bded8b9072846b5ea78e24c264a8725b75000000000001976a914577471a8d634150e4f03009a1b96b0f452857b2188ac788903000000000017a91420ecf8542560a98fc03cd2b293dce8fcb6e7f16387d91a03000000000017a914c35b5fabccfc7ae0b8ef9af141fece952fea74a4877e8f0100000000001976a9141f147ee2b7b8a9afd05c6e1b5ead2e0734445f6788acdfd61600000000001976a91425de320b6dacee557e1e2a2b9745d37b0bf50a8088ace8f70100000000001976a914ff3d033d2be4158b5f6354d718ecc997c180d74188ac11db01000000000017a914dc22d25b894fb0a147d0703676e76e5c9d2c40fe871f5a01000000000017a9140341a9bb1dc7a779c608af8311d0dd3bf69d471787ad5e01000000000017a914b00ee1bb8464b823017c08ac512f43e549d303fc8764e30000000000001976a91492ccbc8395486081f293af4259161d888312b53588ac954104000000000017a914ca0edb486417469fc030c588cb6b0a619fa3b5438702473044022040837acac5fdd9c4a9b1cb2bad845b5651c0a23fa35771298ac10faf91f6dd97022014094fe0d141c74e44c8500e4df7dbf49371fbac68f4ea1415651bdd5a8bf6fa012102fb11000ade2b019fbb1b3ed01bcacbad9c2b57d152f0b77568b66432adf12e0761f208000200000000010120a71236db91bb12476415fd8b01e9ea85b64463619095bee52015160cbca23e0000000017160014fc21c9c6e571a93c90743cd9d79737494c5af731feffffff02038f04000000000017a9144ebfbb1be6a1ca481c42119e936f450c4608bde7872d6f50000000000017a9140524f0e86b2f5da6893a98b8cf17ce9713d097d1870247304402204e16443f2b2797f418cdd628ceb36e8452843222b02b38dec8a665b343693c5302201264f3e9b97316ae9e391bfc69302b077e0d6d82a48ebe30363bae573f5f201c012102c898c7f353c9d1b5a4a63bedcc76869f48aa927383ab9f21538885b2867ca68b61f208000200000000010123526db9a7f1cf27f8f72576654d01a2107623c413a076fa56f34abb76c418290000000017160014f706937a1123307fef6db6e230044ca29c465e0ffeffffff027d6b01000000000017a9144ebfbb1be6a1ca481c42119e936f450c4608bde7871a8225140000000017a914aa3bc6ad61ec104a13f0b701253e6211d1c6ab3f870247304402201c06ddfcf8209afbe1b302861f669043e9b2dd57467f8416261afda57876c5ab022052d124e5dcbe66f03499e3e0ba591d8347c6cded8f808b289609302b5714962e012102b05b8c0f413cac31c4da0f33d9114ff47302f93a59ac9015c9f2e8df89ab49c261f2080002000000000101f5605ac22a3f49a569f817172fda6012bca1ec3af0ec2e0edb546475ab4f9b0f0000000017160014aadda75f66dba0d754b792b51d374ec8142a0f27feffffff0260f900000000000017a9144ebfbb1be6a1ca481c42119e936f450c4608bde7873e266a000000000017a914f7206eea11ba8d578076c8f17707e9ab8d01013b8702473044022036e8e994b979a8dbe3c7f4c81138262ee9b2a8da52013a947926fbeaa723966d02204b739355598582991e03e46ce396e050d314d1e42a7731b9f39864976bdb17df01210262c25a4541e0fd8eb3ec707f9ba4f29993718aa40966cae2dfe3525157d7ae1a61f208000100000001089e99aae4802cb6305cba9ef6de1117e0bbdf96876d9aa65d76e61cffd1d9db010000006b483045022100b6588cdbb1fabd67c4f7d992a6987dd0cc14e7eda582d3211447ca1c85c0ce7002201d4f6b0434bd68dd9f7495bafdca2de306b387052ebbff04f8c08a55fd921b5b012103fe02b06a6bfd9b5a83e0417430a3235a39a0b4c7973591ca27ac58f6474d9120ffffffff0322020000000000001976a9147815270a4e76bc696a066b81ea2ebb04df5c8dfc88ac5b6caa05000000001976a9140a6b825daa1b89f8e100ed8eec6d79b592500f4988ac0000000000000000166a146f6d6e69000000000000001f000000315582b7a700000000010000000113ee4913bfa40812fc17cb843094da0710e6cbfa535d2ca9255156b3922edcdc010000006a47304402205a8d35093a82b23199eb68fe483483b90999c8a7dab99e521c251fb4d0f9a3cf02205ebdd4b48d08d44821c9853248ae20da455a2efc1cc0453f1106d04faca2869d012103fe02b06a6bfd9b5a83e0417430a3235a39a0b4c7973591ca27ac58f6474d9120ffffffff035a33aa05000000001976a9140a6b825daa1b89f8e100ed8eec6d79b592500f4988ac220200000000000017a9143acf353df73a2bd5e30cd13c62236496ac49d1c5870000000000000000166a146f6d6e69000000000000001f000000037e11d60000000000020000000001019996ac2d2d2b6b3f30036da73406937d2675e6af0d9491205286c4048c451c030600000017160014d5aabebbf95aadf34c6880f61d3f412c6c4cc0dffeffffff02c5f040010000000017a9147a7efeed2053e3ec377838d41cb54d5bccbf0ccd8750d57a00000000001976a914c74a512ddc8ce8ecb68cb5d718876c4ff6e8f7a188ac0247304402204f8c937ffcd6f6a99893c79078e7c574af7839f618fc7980a4c4318951c7d98602205769357606bef6bd76704a8db3c618052862c96a8978988eebaf94fc0c15e02b012102628301f0ab5b62d75899fcd4fbe44666ea6b021c268979dd2d74e2953f970ce261f208000200000000010186ec1f0631007ccf9ff41681a907b418c0124a896efdfca727120ec5d1b0247a0100000017160014c4b9ae9551811cd036b2e26c819e363a80576560fdffffff019a05c505000000001976a914959771e05d40423b90ed1b86c4f96d460a426f6788ac024730440220104eeeadf5b6d0511f09f077acea2fe9c2ecc955fa200bd82f68b1d5592c0025022043e3e2a1391e0c72b6037198def96f31f7e20a1f1b04b194cc9dd24a4d0fa682012102427b01f0ae438493ac8b65419f1b5f25f284192730ada2a0567b0ffe4f59781261f208000200000003c80b6179faa2e6aa3748712e769f1d643678db8312742ae719d291d53a409c81000000006b483045022100b06c8c37855761579fbad4379452524b549e76bfb84411f0fe32c839d25eb41702200a2d751eb41bd1c99354d7c791015af2325f72edfbc33dd735df2803c59bbc820121021a74935a8cb2755fcdb2ae045444b30fdb4202456d86dbb5d19e36050974a12afdffffff872fb6eff411eae4d2a035d0534c8ef6270db7b363e7c4cd736029741ddee891000000006a47304402203da1d1f6236c4104de07fa287fc9ebc841dec8346267c9f85374929c5ae3b48502202fbb2eb1e93c3814c1ade00e9c0eefcb16d1fb67e0e2c02b4c7ad4a6b378cde10121021a74935a8cb2755fcdb2ae045444b30fdb4202456d86dbb5d19e36050974a12afdffffff8c40cc59287660d2d08e2ab352e338145dd7a23068f6e70f44d0473cc536cad2000000006b483045022100a113d887a654f806c77d1de81b7636489553fb256d4257581802e56defdb6f0302201990bac55917976cc60f8933f4ccf9cfdc5486a5042b50a9c8e55e6e18d58b3f012103068f5923af03af01b518b4ebe40033d16e0bdc1960cbd25e4bf44d1ff034911ffdffffff01c991c3010000000017a914be44ee84d14295fc30a9e2aff7065b8469e03cb88761f20800010000000001020c31b676dc39e001c0cf6cfae512877969d4a64a30867c141649ec822c13055e0000000017160014a37e6c5f67a0f55fb5e21e145aaa33d9608c8b8affffffffd0c6195fa43f1027667b2cb3668a6f6a3477f3a5e9778f64d521b95a35757a730000000017160014a37e6c5f67a0f55fb5e21e145aaa33d9608c8b8affffffff02fe750400000000001600149a9ea4805e866a25b4a23748084ad4494479d7b980841e000000000017a914bda84c0dac642a7b717120da035d999b30dee8c68702483045022100e080c274bd0d45880303c2483286efbac873768cd6af5cd93ceeade7eb2c000902201fa12d6245e12b157665881f21906707d32874c7fca6cccd19140a927e612437012103b8eac6a7fac022df5120b7386e1b892f9612aed4f196e6733d16d9c0c09fc9ed02473044022028e8bdb40f481de97c23672cd52e2b21ac69681b0b385f3ca2f0a9ec0997b42802204c91efb27d1e6bbdf03e9823c21008c2654723c210f1645a80d3ba0445b316c9012103b8eac6a7fac022df5120b7386e1b892f9612aed4f196e6733d16d9c0c09fc9ed000000000100000001c09db292c2fbb27305e8a0ef4d37ab0260145dbb48c307ed6438ccab75ecc85b010000006b4830450221008184443c904c7231266552143cd1a36b6787c09b0853d4312e16eb9f915ce76f0220033af9aad5bbaaa27776528abf6161ce162edc23e598eca213597bccab81ae61012103f3f44c9e80e2cedc1a2909631a3adea8866ee32187f74d0912387359b0ff36a2ffffffff03fd0faa1e000000001976a914a520c86a08366941cd90d22e11ac1c7eefa2db3788ac22020000000000001976a914ce17a4ecf83746636d1888edd5d7c95a89f84df588ac0000000000000000166a146f6d6e69000000000000001f000000b701403400000000000100000001c4d35c24d18b9ec736dfd2819c1d1ad432aaccfae874148febb36d486f954e5e010000006a47304402203b6ac2873504e746c81b170bbd6e64f0797e0a5b5b677309287df0d923d7a86e02205609844961dfe7b8eaea8ea4d3a66d2fe73e2739ff4cfc1c65f527ebe828234c01210256bfc3543d9c35a6f2fd4b0e80eb100ab32d640d5f0a056751d054f80da5fad1ffffffff02b93c06000000000017a914d53bf35b088c8d8528f46ed4d922a1e8b3d130b5874c2ba700000000001976a914da5451e79564c14b6063d7a03b5556d3a80989d488ac0000000001000000000101a2c4af862fbc6d953048193f7f1f03615eef5fd54df947e053a43113e80426280000000000ffffffff02f4c5bc00000000001600146c217d25d74b16fa9f280337351fca448051e35a188b3c000000000017a914fe13c71cf64050143768d330810607151153a2188702483045022100ca591b4479452cf0f90f975f11753829325848fa3f5ea43c213d2f338265223402204538b16c379912445d7442099d79426a44e1c3198a5ce0b971e0beaef71592760121020daf92c1bbd5a34bd5fc1258bcd8e27ab37bad5c5a8c7430d1ec6da473ff3d9600000000020000000001017a4786327f708efb153aaa407eb7791652032512aad4bf04cd7b992be06778790100000017160014014ed180af0164ede56dc6e29bd2d33a0547c7fcffffffff03220200000000000017a9145351ab5948f0f825120451bcfa89a9291c4ac8b687939107000000000017a9143bbe30bbb17ad46b2ae3c0cb03c3f9bffad2a52a870000000000000000166a146f6d6e69000000000000001f000000003b9aca0002483045022100c298496e4c83baa1d7222a8ce464ceeaeb2f52f76fb2e7bce053a450bbccf3730220147fbbb335e9d7935346bf46fc65573f222e8865b6cf4bd97ca6160a039960bc01210344075fcbb422f9aeb6c27e341cb7eae6f7fe004b2055cd83847494de95e95725000000000100000002121f1d4ac4051675bbd2b790dcc914de0a527504f55f515cb1bea6fa7447922a000000006b483045022100f5b9d282175c0d29ea54732f4cb72cda63a8017eaeae55b23beb0a246710834f02201bf4a6cb87576a1a3ae4f3d429624c88e17442edf59e51a3a4f592bc63028a33012102e47e37af7aaf7dac950d7e488f4c928cbc070b60b0d8298e4896cd398ce4ca04ffffffffc7455c826ab6a05a15193d46cce304acda446d07b93399b3a5cb9dfefa587fde000000006a473044022015100a34ed5641c8c27d3a664a63c95489659ff0ab4ba4923204badc751db4ad02207a2399795bd6ffd62c2cad4bbf00424d7f74e9c06bda8e43bd148530f1030d850121030b901443c508f7317629bc20b3fbad7829164c5a50cafaa4d668db115b35cf6effffffff02faae0600000000001976a914e6f8683e8e51aace4915d81d3410099f1fc0825288ac40420f000000000017a9144a6e6e607cb50282530d711be02a5b7d7a69e314870000000001000000012f1a747763ffa7c64e4036f49abdf11e59cda2911ffbeeaff1aa58d64a38848c010000006a4730440220716974a26d4c6a26b592a48620ee31db11bebfd19f9c5e857ceedf984d2ac0fd022010ae3137343e9afc3fa4ccac86e62213217fa8431b69bd4614ed896d7e6ac968012102d2736cb0d27e499fe516b1496bed1ee17bdd851774b9837c4d3082a8d1550b1dffffffff020c3000000000000017a914b23d12c80a537204ac238c49662cf1c82a144ced8755663e02000000001976a9142029ebbbc384d7a52a07df8ae97baef53171146188ac0000000001000000019fd3d24d43e607ec345b4902f130c393bc634c25d0ab6d2229e21774e1606d99010000006a47304402202ba2c60cbdf3ea31f2afad97a0e2c4c9668da9e0392331cdb6878451bd6e7a0702200f0c472f55a827fd589f4a665a1b07a222261dd13db586fc8d387b773be49023012102c982e321b1acf908b061a57ac60df02dfc96fbe1c782c8906b3d231bf89f0af2ffffffff02271300000000000017a914ded3f657a0800cfbf9e3fad0c3822b20f202ae988770430000000000001976a9145895317b94ff7a60427600b53d7dda9c780d675288ac000000000100000001a4ac137d22835156d7d1ece932f6b8a1c2979158ff40896b2e9218db11c3ed43010000006b483045022100fa45d64c1132be9f5f0b41c060293cfae0baf4bd23c1a1f8a75c7505d6f22a0d02203efe709dbf7399ac86df1d48f4d3163032b12b0309d94735963635567cbc71fc012103c5e8ba59d02fd1f8a38cba1714914f0cdfde69b4c48ceff71939140aed2d10d8ffffffff02271300000000000017a914ded3f657a0800cfbf9e3fad0c3822b20f202ae988746510000000000001976a91414d74ec61144ace85a599baf0bdf1b3bca6803e588ac000000000100000001629fcce7e2d03a694bb1bb1acc8a55550031d6a812e1f26918f0b074762e969a010000006a473044022075ae032d49041e272c8a533126d44cf657569bba127a8b9910f7ee2eca16f16b02200de23b0d5b2e46d371c2bd606f6f850bd3cf600bc0d4946b0e105de880226e570121030473066c8c59fbcce9ffb9b08abe46c08a522dabd05815a59fa5a5654b7a657affffffff02d9e20200000000001976a914134b0f4a28dda067ce26d6e15f893c5b45127f4f88ac9481dc00000000001976a91454dd3d4c8fa3c6ecb98879acbee022cb8b553e6e88ac0000000002000000000101a2e487ebf7b60dd4f9c43ead9cb15c1c300acf9351f26fbe43b2475b00bccf3501000000171600147e2f04296dec5787a0fc14f238a4d7d0ef4db641fdffffff02c2820300000000001976a914538665627664c05352b0531e7da2bc15b70bc0c688ac8bf321000000000017a914f6cbe55365b19300c9bb758d6bbfba27cb5522dc870247304402201403aae38a738cbdeb5c89f757ea92ab907ff4a515d67de5a066915596c246b80220654712b5c105e5bcf364af3e43615e6ed8a8a6ec74b6477d43acfb80f5f692b10121021f043c10668e12fc86bb0656439176fe8dce13751af31c5aa7f2e63b07f3c26b35f20800";
        let block: Block =
            consensus::deserialize(Vec::<u8>::from_hex(blockdump).unwrap().as_slice()).unwrap();
        for (track, tx) in block.txdata.iter().enumerate() {
            let proof = ProvedTransaction::compute_proof(track, &block);
            let pt = ProvedTransaction {
                transaction: tx.clone(),
                merkle_path: proof,
                block_hash: block.header.bitcoin_hash(),
            };
            assert_eq!(pt.merkle_root(), block.header.merkle_root);
        }
    }
}
