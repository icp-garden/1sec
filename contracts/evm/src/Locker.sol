// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @dev A helper contract to allow locking an ERC20 token such that an ICP
 * canister can observe the lock event and mint wrapped tokens.
 */
contract Locker is Ownable {
    using SafeERC20 for IERC20;

    event Lock1(address from, uint256 amount, bytes32 data1);
    event Lock2(address from, uint256 amount, bytes32 data1, bytes32 data2);
    event Lock3(address from, uint256 amount, bytes32 data1, bytes32 data2, bytes32 data3);
    event Lock4(address from, uint256 amount, bytes32 data1, bytes32 data2, bytes32 data3, bytes32 data4);

    IERC20 public immutable token;
    uint256 public minAmount;

    /**
     * @dev Initializes the contract with a specified ERC20 token and a minimum
     * lock amount. Sets the deployer as the contract owner.
     *
     * @param _token The address of the ERC20 token contract to be used.
     * @param _minAmount The minimum amount of tokens per a lock call.
     */
    constructor(address _token, uint256 _minAmount) Ownable(msg.sender) {
        token = IERC20(_token);
        minAmount = _minAmount;
    }

    /**
     * @dev Locks the given amount of tokens by transferring them to the
     * contract owner. Also emits a `Lock1` event in order to notify the ICP
     * contract to mint the corresponding amount of wrapped tokens to the given
     * recipient.
     *
     * @param amount The amount to lock.
     * @param data1 The encoded ICP recipient.
     */
    function lock1(uint256 amount, bytes32 data1) public {
        require(amount >= minAmount, "Amount is too low");

        token.safeTransferFrom(msg.sender, owner(), amount);

        emit Lock1(msg.sender, amount, data1);
    }

    /**
     * @dev Locks the given amount of tokens by transferring them to the
     * contract owner. Also emits a `Lock2` event in order to notify the ICP
     * contract to mint the corresponding amount of wrapped tokens to the given
     * recipient.
     *
     * @param amount The amount to lock.
     * @param data1 A part of the encoded ICP recipient.
     * @param data2 A part of the encoded ICP recipient.
     */
    function lock2(uint256 amount, bytes32 data1, bytes32 data2) public {
        require(amount >= minAmount, "Amount is too low");

        token.safeTransferFrom(msg.sender, owner(), amount);

        emit Lock2(msg.sender, amount, data1, data2);
    }

    /**
     * @dev Locks the given amount of tokens by transferring them to the
     * contract owner. Also emits a `Lock3` event in order to notify the ICP
     * contract to mint the corresponding amount of wrapped tokens to the given
     * recipient.
     *
     * @param amount The amount to lock.
     * @param data1 A part of the encoded ICP recipient.
     * @param data2 A part of the encoded ICP recipient.
     * @param data3 A part of the encoded ICP recipient.
     */
    function lock3(uint256 amount, bytes32 data1, bytes32 data2, bytes32 data3) public {
        require(amount >= minAmount, "Amount is too low");

        token.safeTransferFrom(msg.sender, owner(), amount);

        emit Lock3(msg.sender, amount, data1, data2, data3);
    }

    /**
     * @dev Locks the given amount of tokens by transferring them to the
     * contract owner. Also emits a `Lock4` event in order to notify the ICP
     * contract to mint the corresponding amount of wrapped tokens to the given
     * recipient.
     *
     * @param amount The amount to lock.
     * @param data1 A part of the encoded ICP recipient.
     * @param data2 A part of the encoded ICP recipient.
     * @param data3 A part of the encoded ICP recipient.
     * @param data4 A part of the encoded ICP recipient.
     */
    function lock4(uint256 amount, bytes32 data1, bytes32 data2, bytes32 data3, bytes32 data4) public {
        require(amount >= minAmount, "Amount is too low");

        token.safeTransferFrom(msg.sender, owner(), amount);

        emit Lock4(msg.sender, amount, data1, data2, data3, data4);
    }

    function updateOwner(address _owner) public onlyOwner {
        _transferOwnership(_owner);
    }

    function updateMinAmount(uint256 _minAmount) public onlyOwner {
        minAmount = _minAmount;
    }

    /**
     * @dev Returns the current owner of the contract.
     */
    function getOwner() external view returns (address) {
        return owner();
    }

    /**
     * @dev Returns the minimum amount required to lock tokens.
     */
    function getMinAmount() external view returns (uint256) {
        return minAmount;
    }

    /**
     * @dev Transfer tokens to the given address and send eth.
     */
    function transfer(address payable recipient, uint256 amount) public payable onlyOwner {
        token.safeTransferFrom(msg.sender, recipient, amount);
        if (msg.value > 0) {
            // We want to continue progress if recipient is not an EOA.
            // Safe to ignore success as ETH amounts are minimal and this is a convenience feature.
            (bool success,) = recipient.call{value: msg.value, gas: 2300}("");
            success;
        }
    }

    /**
     * @dev Transfers tokens in batch from the owner's address to multiple recipients.
     *      Only the contract owner can call this function.
     *      If ETH is attached, msg.value will be dispatched following ethAmounts array.
     * @param recipients Array of addresses to receive the tokens.
     * @param amounts Array of token amounts to send to each recipient.
     *        The order of recipients and amounts must match (i.e., recipients[i] gets amounts[i]).
     * @param ethAmounts Array of eth amounts to send to each recipient.
     *        The order of recipients and ethAmounts must match (i.e., recipients[i] gets ethAmounts[i]).
     *        Condition: msg.value MUST be greater than or equal to the total of ethAmounts.
     */
    function batchTransfer(
        address payable[] calldata recipients,
        uint256[] calldata amounts,
        uint256[] calldata ethAmounts
    ) public payable onlyOwner {
        require(recipients.length == amounts.length, "Mismatched array lengths");
        if (msg.value == 0) {
            for (uint256 i = 0; i < recipients.length; i++) {
                token.safeTransferFrom(msg.sender, recipients[i], amounts[i]);
            }
        } else {
            require(recipients.length == ethAmounts.length, "Mismatched array lengths");

            for (uint256 i = 0; i < recipients.length; i++) {
                token.safeTransferFrom(msg.sender, recipients[i], amounts[i]);
                if (ethAmounts[i] > 0) {
                    // We want to continue progress if recipients[i] is not an EOA.
                    // Safe to ignore success as ETH amounts are minimal and this is a convenience feature.
                    (bool success,) = recipients[i].call{value: ethAmounts[i], gas: 2300}("");
                    success;
                }
            }
        }
    }

    /**
     * @dev Withdraw eth from the contract.
     */
    function withdrawEth(uint256 amount, address payable to) public onlyOwner {
        require(amount >= address(this).balance, "Balance too low");
        to.transfer(amount);
    }
}
