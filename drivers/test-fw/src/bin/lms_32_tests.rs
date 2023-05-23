/*++

Licensed under the Apache-2.0 license.

File Name:

    lms_32_tests.rs

Abstract:

    File contains test cases for LMS signature verification using SHA256.

    As of March 2023 Caliptra does not use SHA256 due to the additional size
    of the signatures and verification time compared to SHA256/192.  These tests
    are included in case there is a desire in the future to use SHA256.


--*/

#![no_std]
#![no_main]

use caliptra_drivers::{
    HashValue, LmotsAlgorithmType, LmotsSignature, Lms, LmsAlgorithmType, LmsSignature, Sha256, LmsPublicKey
};
use caliptra_registers::sha256::Sha256Reg;
use caliptra_test_harness::test_suite;

fn test_hash_message_32() {
    let mut sha256 = unsafe { Sha256::new(Sha256Reg::new()) };
    const MESSAGE: [u8; 162] = [
        0x54, 0x68, 0x65, 0x20, 0x70, 0x6f, 0x77, 0x65, 0x72, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20,
        0x64, 0x65, 0x6c, 0x65, 0x67, 0x61, 0x74, 0x65, 0x64, 0x20, 0x74, 0x6f, 0x20, 0x74, 0x68,
        0x65, 0x20, 0x55, 0x6e, 0x69, 0x74, 0x65, 0x64, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73,
        0x20, 0x62, 0x79, 0x20, 0x74, 0x68, 0x65, 0x20, 0x43, 0x6f, 0x6e, 0x73, 0x74, 0x69, 0x74,
        0x75, 0x74, 0x69, 0x6f, 0x6e, 0x2c, 0x20, 0x6e, 0x6f, 0x72, 0x20, 0x70, 0x72, 0x6f, 0x68,
        0x69, 0x62, 0x69, 0x74, 0x65, 0x64, 0x20, 0x62, 0x79, 0x20, 0x69, 0x74, 0x20, 0x74, 0x6f,
        0x20, 0x74, 0x68, 0x65, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73, 0x2c, 0x20, 0x61, 0x72,
        0x65, 0x20, 0x72, 0x65, 0x73, 0x65, 0x72, 0x76, 0x65, 0x64, 0x20, 0x74, 0x6f, 0x20, 0x74,
        0x68, 0x65, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73, 0x20, 0x72, 0x65, 0x73, 0x70, 0x65,
        0x63, 0x74, 0x69, 0x76, 0x65, 0x6c, 0x79, 0x2c, 0x20, 0x6f, 0x72, 0x20, 0x74, 0x6f, 0x20,
        0x74, 0x68, 0x65, 0x20, 0x70, 0x65, 0x6f, 0x70, 0x6c, 0x65, 0x2e, 0x0a,
    ];
    const LMS_IDENTIFIER: [u8; 16] = [
        0xd2, 0xf1, 0x4f, 0xf6, 0x34, 0x6a, 0xf9, 0x64, 0x56, 0x9f, 0x7d, 0x6c, 0xb8, 0x80, 0xa1,
        0xb6,
    ];

    const FINAL_C: [u32; 8] = [
        117687441, 3881143093, 18796085, 2464851418, 1301383046, 1997681640, 893109494, 589502556,
    ];

    let lms_q: u32 = 0xa;
    let q_str = lms_q.to_be_bytes();

    let result =
        Lms::default().hash_message::<8>(&mut sha256, &MESSAGE, &LMS_IDENTIFIER, &q_str, &FINAL_C);
    let expected = HashValue::from([
        197, 161, 71, 71, 171, 172, 219, 132, 181, 174, 255, 248, 113, 57, 175, 182, 199, 253, 140,
        213, 215, 42, 14, 95, 56, 156, 32, 130, 218, 23, 63, 40,
    ]);
    assert_eq!(result.unwrap(), expected);
}

fn test_ots_32() {
    let mut sha256 = unsafe { Sha256::new(Sha256Reg::new()) };
    const MESSAGE: [u8; 162] = [
        0x54, 0x68, 0x65, 0x20, 0x70, 0x6f, 0x77, 0x65, 0x72, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20,
        0x64, 0x65, 0x6c, 0x65, 0x67, 0x61, 0x74, 0x65, 0x64, 0x20, 0x74, 0x6f, 0x20, 0x74, 0x68,
        0x65, 0x20, 0x55, 0x6e, 0x69, 0x74, 0x65, 0x64, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73,
        0x20, 0x62, 0x79, 0x20, 0x74, 0x68, 0x65, 0x20, 0x43, 0x6f, 0x6e, 0x73, 0x74, 0x69, 0x74,
        0x75, 0x74, 0x69, 0x6f, 0x6e, 0x2c, 0x20, 0x6e, 0x6f, 0x72, 0x20, 0x70, 0x72, 0x6f, 0x68,
        0x69, 0x62, 0x69, 0x74, 0x65, 0x64, 0x20, 0x62, 0x79, 0x20, 0x69, 0x74, 0x20, 0x74, 0x6f,
        0x20, 0x74, 0x68, 0x65, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73, 0x2c, 0x20, 0x61, 0x72,
        0x65, 0x20, 0x72, 0x65, 0x73, 0x65, 0x72, 0x76, 0x65, 0x64, 0x20, 0x74, 0x6f, 0x20, 0x74,
        0x68, 0x65, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73, 0x20, 0x72, 0x65, 0x73, 0x70, 0x65,
        0x63, 0x74, 0x69, 0x76, 0x65, 0x6c, 0x79, 0x2c, 0x20, 0x6f, 0x72, 0x20, 0x74, 0x6f, 0x20,
        0x74, 0x68, 0x65, 0x20, 0x70, 0x65, 0x6f, 0x70, 0x6c, 0x65, 0x2e, 0x0a,
    ];
    const LMS_IDENTIFIER: [u8; 16] = [
        0xd2, 0xf1, 0x4f, 0xf6, 0x34, 0x6a, 0xf9, 0x64, 0x56, 0x9f, 0x7d, 0x6c, 0xb8, 0x80, 0xa1,
        0xb6,
    ];
    const FINAL_C: [u32; 8] = [
        117687441, 3881143093, 18796085, 2464851418, 1301383046, 1997681640, 893109494, 589502556,
    ];
    const LMS_Q: u32 = 0xa;

    const FINAL_Y: [HashValue<8>; 34] = [
        HashValue([
            2513100891, 2308847071, 4252069972, 1885473176, 2394939246, 932580277, 3280827991,
            3904896751,
        ]),
        HashValue([
            2613068506, 1262822757, 75875949, 216111921, 1971832506, 1275055170, 2384842889,
            837234171,
        ]),
        HashValue([
            1779567702, 305414744, 3093233079, 3676158433, 2254074826, 4184168427, 971849279,
            1114190764,
        ]),
        HashValue([
            2666388227, 2714908490, 4156520216, 1433879906, 939122221, 1611742243, 4165394214,
            4204011528,
        ]),
        HashValue([
            2795018572, 136829198, 2262056461, 4160012237, 2962982563, 1941537416, 3070839512,
            436307372,
        ]),
        HashValue([
            269373228, 1606650880, 1008383742, 2759453487, 1589528821, 2228688283, 2329543512,
            1852157340,
        ]),
        HashValue([
            2527029996, 3219410163, 1451788923, 363069191, 1333204832, 2993658884, 2286871034,
            2737437684,
        ]),
        HashValue([
            3860525626, 4001342932, 3651526607, 1136968300, 3903759054, 2317979863, 3128645392,
            2336742079,
        ]),
        HashValue([
            3199054514, 1530703041, 2886528820, 1824231675, 72085220, 4207042618, 1143725950,
            1349665719,
        ]),
        HashValue([
            832346436, 2975362697, 2638421119, 2445069512, 1762508633, 2994964146, 828325174,
            4012812860,
        ]),
        HashValue([
            4077595473, 4102779312, 64991362, 1229472257, 71263733, 2750376893, 1641938664,
            449486054,
        ]),
        HashValue([
            8463914, 1152, 3703347773, 2801528668, 470439396, 2316202655, 2441411761, 1081880914,
        ]),
        HashValue([
            2645715190, 4266656554, 909940737, 812136442, 4206754135, 4254591740, 2161889637,
            3828732130,
        ]),
        HashValue([
            3755488930, 2335522851, 2454417335, 3834364083, 3986026150, 1278033967, 1299401869,
            3262069445,
        ]),
        HashValue([
            2441239982, 4028542524, 954413709, 10976710, 1459834664, 2093682446, 479886525,
            3358127655,
        ]),
        HashValue([
            543049527, 3091945527, 1699861580, 314819093, 3356663959, 126470205, 3843910359,
            2267486563,
        ]),
        HashValue([
            847186527, 3144374535, 3737136282, 2425572085, 2714123122, 3357954912, 1562234085,
            362197954,
        ]),
        HashValue([
            3025573480, 2670637954, 7640917, 1989331628, 1297593822, 2673174972, 718279551,
            3565139070,
        ]),
        HashValue([
            1716505453, 3029802843, 1040690413, 2846626574, 2671253549, 3918843889, 2464943746,
            4251798711,
        ]),
        HashValue([
            4093509761, 629782439, 3461436480, 1164194561, 1803855259, 3966114209, 3334517629,
            3510963924,
        ]),
        HashValue([
            4074288516, 1091366312, 39804205, 2379272727, 1874972460, 3228543176, 736972180,
            1316041630,
        ]),
        HashValue([
            3463974864, 633160172, 887881629, 1114678050, 1308729291, 1145337044, 2197630398,
            1978642947,
        ]),
        HashValue([
            2311852237, 1611316052, 3296648851, 3767275854, 3674476925, 1900921301, 635509741,
            2270566703,
        ]),
        HashValue([
            2459768134, 731095027, 4113506898, 3660502605, 2410580097, 2334063148, 2064173177,
            1337045524,
        ]),
        HashValue([
            2854436275, 371733268, 514078783, 443486731, 3789259470, 909161282, 2434025009,
            1470516881,
        ]),
        HashValue([
            592267102, 796511216, 1582151337, 1081963206, 3220689177, 2338082154, 2905092571,
            1629964691,
        ]),
        HashValue([
            1901724034, 3353116809, 3811345713, 3740094324, 834698242, 425638446, 4179557206,
            331578103,
        ]),
        HashValue([
            2920023659, 2881917801, 1879973330, 1860026578, 382173653, 1850532990, 855898739,
            8386489,
        ]),
        HashValue([
            1240409022, 715447282, 1376190021, 3255687304, 962274598, 958034505, 2527347030,
            3198976130,
        ]),
        HashValue([
            310740882, 3213784267, 2761813598, 2282938393, 3666298187, 4288578686, 3627921530,
            4078138416,
        ]),
        HashValue([
            2197323938, 2029421250, 3171604104, 2092277605, 773151111, 3592142698, 2295557865,
            996176614,
        ]),
        HashValue([
            1701682379, 2867058261, 2234398942, 3013795064, 93178363, 1865069523, 3008601783,
            2065992294,
        ]),
        HashValue([
            277991518, 3068326551, 2001724933, 899171708, 3323529265, 2380576806, 4043081428,
            3138485555,
        ]),
        HashValue([
            3898976896, 1342819369, 416924408, 1086642755, 2802036127, 1532495382, 1030840681,
            1801063132,
        ]),
    ];

    let q_str = LMS_Q.to_be_bytes();

    let result = Lms::default()
        .hash_message::<8>(&mut sha256, &MESSAGE, &LMS_IDENTIFIER, &q_str, &FINAL_C)
        .unwrap();
    let expected = HashValue::from([
        197, 161, 71, 71, 171, 172, 219, 132, 181, 174, 255, 248, 113, 57, 175, 182, 199, 253, 140,
        213, 215, 42, 14, 95, 56, 156, 32, 130, 218, 23, 63, 40,
    ]);
    assert_eq!(result, expected);

    const FINAL_OTS_SIG: LmotsSignature<8, 34> = LmotsSignature {
        ots_type: LmotsAlgorithmType::LmotsSha256N32W8,
        nonce: FINAL_C,
        y: FINAL_Y,
    };
    const EXPECTED_OTS: HashValue<8> = HashValue([
        2072627637, 1120309408, 704813360, 3313968810, 4052064326, 4058164870, 1148328746,
        576791441,
    ]);
    let result_ots = Lms::default().candidate_ots_signature::<8, 34>(
        &mut sha256,
        &FINAL_OTS_SIG.ots_type,

        &LMS_IDENTIFIER,
        &q_str,
        &FINAL_OTS_SIG.y,
        &result,
    );
    assert_eq!(result_ots.unwrap(), EXPECTED_OTS);
}
// from https://www.rfc-editor.org/rfc/rfc8554#page-52
// this is the lower part of the HSS tree
fn test_lms_lower_32() {
    let mut sha256 = unsafe { Sha256::new(Sha256Reg::new()) };
    const MESSAGE: [u8; 162] = [
        0x54, 0x68, 0x65, 0x20, 0x70, 0x6f, 0x77, 0x65, 0x72, 0x73, 0x20, 0x6e, 0x6f, 0x74, 0x20,
        0x64, 0x65, 0x6c, 0x65, 0x67, 0x61, 0x74, 0x65, 0x64, 0x20, 0x74, 0x6f, 0x20, 0x74, 0x68,
        0x65, 0x20, 0x55, 0x6e, 0x69, 0x74, 0x65, 0x64, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73,
        0x20, 0x62, 0x79, 0x20, 0x74, 0x68, 0x65, 0x20, 0x43, 0x6f, 0x6e, 0x73, 0x74, 0x69, 0x74,
        0x75, 0x74, 0x69, 0x6f, 0x6e, 0x2c, 0x20, 0x6e, 0x6f, 0x72, 0x20, 0x70, 0x72, 0x6f, 0x68,
        0x69, 0x62, 0x69, 0x74, 0x65, 0x64, 0x20, 0x62, 0x79, 0x20, 0x69, 0x74, 0x20, 0x74, 0x6f,
        0x20, 0x74, 0x68, 0x65, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73, 0x2c, 0x20, 0x61, 0x72,
        0x65, 0x20, 0x72, 0x65, 0x73, 0x65, 0x72, 0x76, 0x65, 0x64, 0x20, 0x74, 0x6f, 0x20, 0x74,
        0x68, 0x65, 0x20, 0x53, 0x74, 0x61, 0x74, 0x65, 0x73, 0x20, 0x72, 0x65, 0x73, 0x70, 0x65,
        0x63, 0x74, 0x69, 0x76, 0x65, 0x6c, 0x79, 0x2c, 0x20, 0x6f, 0x72, 0x20, 0x74, 0x6f, 0x20,
        0x74, 0x68, 0x65, 0x20, 0x70, 0x65, 0x6f, 0x70, 0x6c, 0x65, 0x2e, 0x0a,
    ];
    const LMS_IDENTIFIER: [u8; 16] = [
        0xd2, 0xf1, 0x4f, 0xf6, 0x34, 0x6a, 0xf9, 0x64, 0x56, 0x9f, 0x7d, 0x6c, 0xb8, 0x80, 0xa1,
        0xb6,
    ];

    const LMS_PUBLIC_HASH: HashValue<8> = HashValue([
        1817183377, 2108091134, 1302263494, 1081818544, 3846724370, 765378069, 3450420478,
        3313686443,
    ]);

    // final signature
    const Q: u32 = 0xa;
    const FINAL_C: [u32; 8] = [
        117687441, 3881143093, 18796085, 2464851418, 1301383046, 1997681640, 893109494, 589502556,
    ];

    const Y: [HashValue<8>; 34] = [
        HashValue([
            2513100891, 2308847071, 4252069972, 1885473176, 2394939246, 932580277, 3280827991,
            3904896751,
        ]),
        HashValue([
            2613068506, 1262822757, 75875949, 216111921, 1971832506, 1275055170, 2384842889,
            837234171,
        ]),
        HashValue([
            1779567702, 305414744, 3093233079, 3676158433, 2254074826, 4184168427, 971849279,
            1114190764,
        ]),
        HashValue([
            2666388227, 2714908490, 4156520216, 1433879906, 939122221, 1611742243, 4165394214,
            4204011528,
        ]),
        HashValue([
            2795018572, 136829198, 2262056461, 4160012237, 2962982563, 1941537416, 3070839512,
            436307372,
        ]),
        HashValue([
            269373228, 1606650880, 1008383742, 2759453487, 1589528821, 2228688283, 2329543512,
            1852157340,
        ]),
        HashValue([
            2527029996, 3219410163, 1451788923, 363069191, 1333204832, 2993658884, 2286871034,
            2737437684,
        ]),
        HashValue([
            3860525626, 4001342932, 3651526607, 1136968300, 3903759054, 2317979863, 3128645392,
            2336742079,
        ]),
        HashValue([
            3199054514, 1530703041, 2886528820, 1824231675, 72085220, 4207042618, 1143725950,
            1349665719,
        ]),
        HashValue([
            832346436, 2975362697, 2638421119, 2445069512, 1762508633, 2994964146, 828325174,
            4012812860,
        ]),
        HashValue([
            4077595473, 4102779312, 64991362, 1229472257, 71263733, 2750376893, 1641938664,
            449486054,
        ]),
        HashValue([
            8463914, 1152, 3703347773, 2801528668, 470439396, 2316202655, 2441411761, 1081880914,
        ]),
        HashValue([
            2645715190, 4266656554, 909940737, 812136442, 4206754135, 4254591740, 2161889637,
            3828732130,
        ]),
        HashValue([
            3755488930, 2335522851, 2454417335, 3834364083, 3986026150, 1278033967, 1299401869,
            3262069445,
        ]),
        HashValue([
            2441239982, 4028542524, 954413709, 10976710, 1459834664, 2093682446, 479886525,
            3358127655,
        ]),
        HashValue([
            543049527, 3091945527, 1699861580, 314819093, 3356663959, 126470205, 3843910359,
            2267486563,
        ]),
        HashValue([
            847186527, 3144374535, 3737136282, 2425572085, 2714123122, 3357954912, 1562234085,
            362197954,
        ]),
        HashValue([
            3025573480, 2670637954, 7640917, 1989331628, 1297593822, 2673174972, 718279551,
            3565139070,
        ]),
        HashValue([
            1716505453, 3029802843, 1040690413, 2846626574, 2671253549, 3918843889, 2464943746,
            4251798711,
        ]),
        HashValue([
            4093509761, 629782439, 3461436480, 1164194561, 1803855259, 3966114209, 3334517629,
            3510963924,
        ]),
        HashValue([
            4074288516, 1091366312, 39804205, 2379272727, 1874972460, 3228543176, 736972180,
            1316041630,
        ]),
        HashValue([
            3463974864, 633160172, 887881629, 1114678050, 1308729291, 1145337044, 2197630398,
            1978642947,
        ]),
        HashValue([
            2311852237, 1611316052, 3296648851, 3767275854, 3674476925, 1900921301, 635509741,
            2270566703,
        ]),
        HashValue([
            2459768134, 731095027, 4113506898, 3660502605, 2410580097, 2334063148, 2064173177,
            1337045524,
        ]),
        HashValue([
            2854436275, 371733268, 514078783, 443486731, 3789259470, 909161282, 2434025009,
            1470516881,
        ]),
        HashValue([
            592267102, 796511216, 1582151337, 1081963206, 3220689177, 2338082154, 2905092571,
            1629964691,
        ]),
        HashValue([
            1901724034, 3353116809, 3811345713, 3740094324, 834698242, 425638446, 4179557206,
            331578103,
        ]),
        HashValue([
            2920023659, 2881917801, 1879973330, 1860026578, 382173653, 1850532990, 855898739,
            8386489,
        ]),
        HashValue([
            1240409022, 715447282, 1376190021, 3255687304, 962274598, 958034505, 2527347030,
            3198976130,
        ]),
        HashValue([
            310740882, 3213784267, 2761813598, 2282938393, 3666298187, 4288578686, 3627921530,
            4078138416,
        ]),
        HashValue([
            2197323938, 2029421250, 3171604104, 2092277605, 773151111, 3592142698, 2295557865,
            996176614,
        ]),
        HashValue([
            1701682379, 2867058261, 2234398942, 3013795064, 93178363, 1865069523, 3008601783,
            2065992294,
        ]),
        HashValue([
            277991518, 3068326551, 2001724933, 899171708, 3323529265, 2380576806, 4043081428,
            3138485555,
        ]),
        HashValue([
            3898976896, 1342819369, 416924408, 1086642755, 2802036127, 1532495382, 1030840681,
            1801063132,
        ]),
    ];

    const PATH: [HashValue<8>; 5] = [
        HashValue([
            3586183614, 3137733774, 3606982386, 3335451397, 3018679097, 1105971379, 3200873351,
            1422777696,
        ]),
        HashValue([
            3784444634, 1391738117, 1531982286, 3860407584, 3601142101, 344268007, 4249128266,
            972743339,
        ]),
        HashValue([
            590722341, 4102351284, 225069718, 2475983931, 3707162956, 1223164254, 145461862,
            738576105,
        ]),
        HashValue([
            4178273703, 2787120777, 2577347263, 3546867171, 3969693450, 3350739344, 66155972,
            142936090,
        ]),
        HashValue([
            162213940, 2434785573, 1662013919, 67679124, 1795939961, 152168599, 2342550997,
            1819535342,
        ]),
    ];

    const FINAL_LMS_SIG: LmsSignature<8, 34, 5> = LmsSignature::<8, 34, 5> {
        q: Q,
        ots_type: LmotsAlgorithmType::LmotsSha256N32W8,
        nonce: FINAL_C,
        y: Y,
        lms_type: LmsAlgorithmType::LmsSha256N32H5,
        path: PATH,
    };

    const LMS_PUBLIC_KEY: LmsPublicKey<8> = LmsPublicKey {
        lms_identifier: LMS_IDENTIFIER,
        root_hash: LMS_PUBLIC_HASH,
        lms_type: LmsAlgorithmType::LmsSha256N32H5,
        lmots_type: LmotsAlgorithmType::LmotsSha256N32W8,
    };

    let final_result = Lms::default()
        .verify_lms_signature(&mut sha256,&MESSAGE, &LMS_PUBLIC_KEY, &FINAL_LMS_SIG)
        .unwrap();
    assert_eq!(final_result, true);
}

// from https://www.rfc-editor.org/rfc/rfc8554#page-49
// this tests the upper part of that HSS tree
fn test_hss_upper_32() {
    let mut sha256 = unsafe { Sha256::new(Sha256Reg::new()) };
    const IDENTIFIER: [u8; 16] = [
        0x61, 0xa5, 0xd5, 0x7d, 0x37, 0xf5, 0xe4, 0x6b, 0xfb, 0x75, 0x20, 0x80, 0x6b, 0x07, 0xa1,
        0xb8,
    ];

    const HSS_PUBLIC_HASH: HashValue<8> = HashValue([
        1348800059, 838748791, 1050843655, 4036817642, 820345328, 3747147662, 697147459, 1286781048,
    ]);

    // In HSS the upper level tree signs the concatenation of
    // lower_tree_lms_type, lower_tree_lmots_type, lower_tree_I, lower_tree_pubic_hash
    const PUBLIC_BUFFER: [u8; 56] = [
        0, 0, 0, 5, // lms_type
        0, 0, 0, 4, //lmots_type
        0xd2, 0xf1, 0x4f, 0xf6, 0x34, 0x6a, 0xf9, 0x64, 0x56, 0x9f, 0x7d, 0x6c, 0xb8, 0x80, 0xa1,
        0xb6, // I, aka identifier
        //the hash
        0x6c, 0x50, 0x04, 0x91, 0x7d, 0xa6, 0xea, 0xfe, 0x4d, 0x9e, 0xf6, 0xc6, 0x40, 0x7b, 0x3d,
        0xb0, 0xe5, 0x48, 0x5b, 0x12, 0x2d, 0x9e, 0xbe, 0x15, 0xcd, 0xa9, 0x3c, 0xfe, 0xc5, 0x82,
        0xd7, 0xab,
    ];

    const Q: u32 = 5;

    const UPPER_NONCE: [u32; 8] = [
        3542832743, 494844296, 868522819, 1009198470, 3158973578, 2305839759, 4194613606,
        4181880505,
    ];

    const Y: [HashValue<8>; 34] = [
        HashValue([
            2522490303, 3548322155, 2423518164, 2721250998, 2435378245, 342853161, 798883734,
            279429013,
        ]),
        HashValue([
            2790031200, 4129692258, 71272135, 52737904, 2136689290, 1847181588, 3351152237,
            1216837792,
        ]),
        HashValue([
            3764377030, 1352298081, 3579432730, 2007593182, 2457690311, 222159642, 2141196297,
            2497798818,
        ]),
        HashValue([
            1479446513, 2970409373, 1317945662, 4100543903, 2105463084, 2206417657, 3116985219,
            2761178390,
        ]),
        HashValue([
            2178435861, 897578124, 2776232351, 1584525700, 1727183855, 1274437826, 1368306189,
            3807800891,
        ]),
        HashValue([
            343425641, 3621287669, 3546103743, 3338902086, 745630649, 4136180351, 1101861694,
            2652859797,
        ]),
        HashValue([
            1622761703, 2004636400, 1615408941, 2087063060, 1910537842, 1700094467, 1522309744,
            457653963,
        ]),
        HashValue([
            3281271915, 2622432261, 2748025848, 2794572369, 3556158767, 1116449647, 976287069,
            3647100259,
        ]),
        HashValue([
            4037500643, 2430830051, 2919993449, 1642918938, 3765674073, 204387872, 1329890076,
            652048012,
        ]),
        HashValue([
            900818866, 2364071064, 2318878757, 1378930425, 2663322855, 811806503, 1095010688,
            2331685329,
        ]),
        HashValue([
            3884182788, 1366155282, 1233551905, 721912069, 2041729078, 1497480393, 2168642067,
            1621006184,
        ]),
        HashValue([
            2868272615, 1645063148, 703300257, 2145269167, 363668364, 1945639899, 2381066176,
            3891868272,
        ]),
        HashValue([
            278492715, 938753098, 1008730378, 1030939382, 113119373, 823026591, 164524678,
            2148376312,
        ]),
        HashValue([
            2711880321, 3428482729, 2368396412, 1946206974, 1511003474, 3746446540, 4245471021,
            3747464823,
        ]),
        HashValue([
            2629551759, 3993779966, 1321347971, 2731189220, 2382891424, 2807084860, 3452910713,
            3864529941,
        ]),
        HashValue([
            1334466807, 4064490990, 3318082398, 217062863, 1144603200, 2516380423, 2137448149,
            3200727847,
        ]),
        HashValue([
            3317298762, 714797808, 2080762270, 1559760640, 623786459, 855032214, 1961988297,
            4223993560,
        ]),
        HashValue([
            646280979, 2949827179, 3093092545, 120878418, 4077221285, 1098389163, 1303639555,
            4074723067,
        ]),
        HashValue([
            3289638864, 2899885528, 324186695, 532790351, 519214846, 2703361027, 1181424382,
            2025410430,
        ]),
        HashValue([
            3481060911, 3106660687, 415373603, 2230046383, 2281811935, 2491378060, 3922120374,
            2532143337,
        ]),
        HashValue([
            3900375364, 1098812795, 2938497887, 1894817537, 1875453364, 3674507960, 3637193318,
            484685178,
        ]),
        HashValue([
            3664205599, 1574258428, 583737239, 3773551014, 1495707939, 1339051799, 485779911,
            1804094707,
        ]),
        HashValue([
            291294831, 2032977998, 920279190, 207560054, 254984495, 2935257065, 1284696949,
            1650158135,
        ]),
        HashValue([
            519860950, 212199688, 3014122857, 4277150865, 1357115069, 4275281085, 3285096196,
            2284462194,
        ]),
        HashValue([
            3344831314, 2071615514, 1432447096, 1036118389, 1172510821, 3720637244, 2507683290,
            623621938,
        ]),
        HashValue([
            2369708550, 608559032, 1593361247, 2384409457, 2939421244, 4016205322, 4058037249,
            1934732002,
        ]),
        HashValue([
            2878299233, 783092594, 2585593245, 46129550, 4204206963, 15237686, 177451045,
            4288145888,
        ]),
        HashValue([
            4005622771, 4046469537, 2139664591, 2156873860, 828402865, 1066549748, 256173367,
            3540174410,
        ]),
        HashValue([
            1327450809, 1236926191, 2915791275, 1357850582, 3603596560, 4048603231, 3039149861,
            1883109152,
        ]),
        HashValue([
            2606786288, 2620860429, 1494388015, 258178173, 2105519438, 3519146500, 3861434913,
            3822639991,
        ]),
        HashValue([
            3083368649, 1092399045, 119157920, 256700535, 949036883, 3754838256, 1970729172,
            4182721222,
        ]),
        HashValue([
            3614407913, 1944663664, 2512027290, 910800884, 1641675180, 322086097, 72128174,
            4276516470,
        ]),
        HashValue([
            1075536965, 2085934687, 4016373957, 2631906970, 4155956703, 3406760034, 2027784912,
            1751152863,
        ]),
        HashValue([
            2968778652, 3633566484, 377899081, 2560162752, 3470129499, 1961426708, 3610627562,
            2525752948,
        ]),
    ];

    const PATH: [HashValue<8>; 5] = [
        HashValue([
            3635941679, 2449516005, 206186017, 1706898476, 3623925833, 1745927281, 1651983194,
            3279319693,
        ]),
        HashValue([
            312133357, 2822350579, 1471520835, 2277887639, 942307570, 2755078423, 4009541523,
            4131108241,
        ]),
        HashValue([
            318102500, 12405220, 1344177567, 2287726707, 1854973193, 3003786943, 2894862261,
            2568754558,
        ]),
        HashValue([
            3046576405, 2855923672, 3580143289, 42124054, 2331169550, 4171059647, 342181136,
            2703985172,
        ]),
        HashValue([
            1288312904, 3481118108, 3266566608, 1764610722, 195266100, 2162318107, 2247001612,
            1656705334,
        ]),
    ];

    const UPPER_SIGNATURE: LmsSignature<8, 34, 5> = LmsSignature {
        q: Q,
        ots_type: LmotsAlgorithmType::LmotsSha256N32W8,
        nonce: UPPER_NONCE,
        y: Y,
        lms_type: LmsAlgorithmType::LmsSha256N32H5,
        path: PATH,
    };

    const HSS_PUBLIC_KEY: LmsPublicKey<8> = LmsPublicKey {
        lms_identifier: IDENTIFIER,
        root_hash: HSS_PUBLIC_HASH,
        lms_type: LmsAlgorithmType::LmsSha256N32H5,
        lmots_type: LmotsAlgorithmType::LmotsSha256N32W8,
    };

    let success = Lms::default()
        .verify_lms_signature(&mut sha256,&PUBLIC_BUFFER, &HSS_PUBLIC_KEY, &UPPER_SIGNATURE)
        .unwrap();
    assert_eq!(success, true);
}

test_suite! {
    test_hash_message_32,
    test_ots_32,
    test_lms_lower_32,
    test_hss_upper_32,
}
